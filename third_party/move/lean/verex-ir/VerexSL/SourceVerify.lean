-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexSL
import VerexSL.Proofs

/-!
# Verifying a source through its VerexSL rendering

A frontend's validated unit is rendered as VerexSL, written out, and
elaborated in this process, which verifies every specified function. The
rendering records the sources it was made from (`verex sources at`). Files
authored beside a source accompany the namespace rendered from it, whose
`proof_file` pragma names the file:

- a Rust file's specifications, its `.spec.lean`, are VerexSL items of the
  namespace, spliced into its rendering; a module file's reopen the module's
  scope (`mod a where`);
- a Move file's proofs, its `.proof.lean`, are a Lean file of their own
  (`VerexSL.Proofs`). The namespace is registered with its verification
  deferred, and the proof file, elaborated over what its command left,
  verifies it with the file's items, as the editor elaborates the file.

Every message is reported in the coordinates of the file it came from. A
message in a proof file is at its own position. A position in spliced items
maps line by line to their file. Any other position in the rendering maps
through the correspondence between the source's unit and the unit elaborated
from the rendering: the two are aligned declaration by declaration and then
node by node while their shapes agree, and a position takes the source range
of the innermost aligned node around it. A position outside every aligned node
stays in the generated file.
-/

namespace VerexSL.SourceVerify

open Lean VerexIR VerexIR.Validation

/-- How a file authored beside a source accompanies its namespace. -/
inductive CompanionRole where
  /-- VerexSL items of the namespace, spliced into its rendering: a Rust
  file's specifications. -/
  | spliced
  /-- A proof file verifying the namespace after its registration: a Move
  file's proofs. -/
  | proofs
  deriving BEq

/-- A file authored beside a source, accompanying the namespace rendered from
it. The frontend names the file on the namespace's `proof_file` pragma
whether or not it exists, so a message asking for a proof can say where to
write it. -/
structure Companion where
  path : System.FilePath
  namespaceId : NamespaceId
  /-- The scope of the namespace its items belong to: the module a Rust module
  file declares, empty for a crate root or a single file. -/
  scope : Array String := #[]
  /-- The file's text; `none` when the file does not exist. -/
  text : Option String := none
  /-- Synthesized items retain coordinates in the generated rendering, which
  exists on disk, rather than pointing at an imaginary companion file. -/
  generated : Bool := false
  role : CompanionRole := .spliced
  /-- The other namespaces its items name, which their namespace follows. -/
  uses : Array NamespaceId := #[]

/-- One namespace verified over the compiled namespaces rendered before it. -/
structure CompiledNamespaces where
  /-- The search path entry of the compiled namespaces. -/
  root : System.FilePath
  /-- The namespace verified. -/
  target : NamespaceId

structure Request where
  unit : ValidatedUnit
  companions : Array Companion := #[]
  /-- The directory the unit's relative source names are relative to: a
  Cargo workspace's root; the current directory without one. -/
  sourceRoot : Option System.FilePath := none
  /-- Where the generated VerexSL is written. -/
  output : System.FilePath
  /-- Write the rendering only, without elaborating it: for inspection and
  for tools that elaborate its parts themselves. -/
  renderOnly : Bool := false
  /-- The heartbeat budget of a function's verification where no `pragma
  heartbeats` sets it, in thousands of `maxHeartbeats` units; the default of
  `verex.verifyHeartbeats` without one. -/
  heartbeats : Option Nat := none
  /-- Verify every function automatically, its authored proof ignored
  (`verex.ignoreProofs`). -/
  ignoreProofs : Bool := false
  /-- How many targets of a module are proved at once (`verex.jobs`); the
  option's default without one. -/
  jobs : Option Nat := none
  /-- Import the namespaces before the target rather than elaborating them. -/
  compiled : Option CompiledNamespaces := none

/-- A message in the coordinates of the file it is attributed to; lines and
columns count from one. -/
structure Report where
  file : String
  line : Nat
  column : Nat
  severity : MessageSeverity
  text : String

def Report.render (report : Report) : String :=
  let severity := match report.severity with
    | .error => "error"
    | .warning => "warning"
    | .information => "info"
  s!"{report.file}:{report.line}:{report.column}: {severity}: {report.text}"

/-! ## Alignment -/

/-- Pairs of a generated location with the original location it renders. -/
private abbrev AlignM := StateM (Array (LocId × LocId))

private def pair (generated original : LocId) : AlignM Unit :=
  modify (·.push (generated, original))

/-- A declaration's name and the scope it is in. -/
private def nameOf? (ns : ValidatedNamespace) (id : NameId) : Option (Array String × String) :=
  (ns.nameOf id).map fun name => (name.scope, name.name)

/-- A block's statements followed by its result, as one sequence: a trailing
statement may come back as the result. An empty unit result, which a
rendering elides, is no item. -/
private def blockItems (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Array ExprId :=
  match result.bind (ns.expressions[·.index]?) with
  | some { kind := .value .unit _, .. }
  | some { kind := .operation (.primitive .tuple) _ #[] _, .. }
  | some { kind := .block #[] none, .. } => statements
  | _ => statements ++ result.toArray

mutual

private partial def alignExpr (g o : ValidatedNamespace) (generated original : ExprId) : AlignM Unit := do
  let some ge := g.expressions[generated.index]? | return
  let some oe := o.expressions[original.index]? | return
  pair ge.loc oe.loc
  -- A block of one item renders as the item.
  match ge.kind, oe.kind with
  | .block gStatements gResult, kind =>
      if !(kind matches .block ..) then
        if let #[item] := blockItems g gStatements gResult then
          return ← alignExpr g o item original
  | kind, .block oStatements oResult =>
      if !(kind matches .block ..) then
        if let #[item] := blockItems o oStatements oResult then
          return ← alignExpr g o generated item
  | _, _ => pure ()
  match ge.kind, oe.kind with
  | .operation _ _ gArgs _, .operation _ _ oArgs _ => alignExprs g o gArgs oArgs
  | .block gStatements gResult, .block oStatements oResult =>
      alignExprs g o (blockItems g gStatements gResult) (blockItems o oStatements oResult)
  | .letDecl gPattern gValue gBody, .letDecl oPattern oValue oBody =>
      alignPattern g o gPattern oPattern
      alignOption g o gValue oValue
      alignExpr g o gBody oBody
  | .ifElse gCondition gThen gElse, .ifElse oCondition oThen oElse =>
      alignExpr g o gCondition oCondition
      alignExpr g o gThen oThen
      alignOption g o gElse oElse
  | .match_ gScrutinee gArms, .match_ oScrutinee oArms =>
      alignExpr g o gScrutinee oScrutinee
      if gArms.size == oArms.size then
        for (gArm, oArm) in gArms.zip oArms do
          alignPattern g o gArm.pattern oArm.pattern
          alignOption g o gArm.guard oArm.guard
          alignExpr g o gArm.body oArm.body
  | .loop _ gBody, .loop _ oBody => alignExpr g o gBody oBody
  | .break_ _ gValue, .break_ _ oValue => alignOption g o gValue oValue
  | .return_ gValues, .return_ oValues => alignExprs g o gValues oValues
  | .throw_ _ gArgs, .throw_ _ oArgs => alignExprs g o gArgs oArgs
  | .assign gPlace gValue, .assign oPlace oValue =>
      alignPlace g o gPlace oPlace
      alignExpr g o gValue oValue
  | .assignPattern gPattern gValue, .assignPattern oPattern oValue =>
      alignPattern g o gPattern oPattern
      alignExpr g o gValue oValue
  -- An assignment to one variable renders as either form.
  | .assign _ gValue, .assignPattern _ oValue | .assignPattern _ gValue, .assign _ oValue =>
      alignExpr g o gValue oValue
  | .quantifier _ gBinders _ gCondition gBody, .quantifier _ oBinders _ oCondition oBody =>
      if gBinders.size == oBinders.size then
        for (gBinder, oBinder) in gBinders.zip oBinders do
          alignPattern g o gBinder.pattern oBinder.pattern
          alignExpr g o gBinder.domain oBinder.domain
      alignOption g o gCondition oCondition
      alignExpr g o gBody oBody
  | .spec gBlock, .spec oBlock =>
      pair gBlock.loc oBlock.loc
      alignConditions g o gBlock.conditions oBlock.conditions
      match gBlock.frame, oBlock.frame with
      | some gFrame, some oFrame => alignExprs g o gFrame.modifies oFrame.modifies
      | _, _ => pure ()
  | _, _ => pure ()

private partial def alignExprs (g o : ValidatedNamespace) (generated original : Array ExprId) :
    AlignM Unit := do
  if generated.size == original.size then
    for (gExpr, oExpr) in generated.zip original do alignExpr g o gExpr oExpr

private partial def alignOption (g o : ValidatedNamespace) (generated original : Option ExprId) :
    AlignM Unit := do
  if let (some gExpr, some oExpr) := (generated, original) then alignExpr g o gExpr oExpr

private partial def alignPattern (g o : ValidatedNamespace) (generated original : PatternId) :
    AlignM Unit := do
  let some gp := g.patterns[generated.index]? | return
  let some op := o.patterns[original.index]? | return
  pair gp.loc op.loc
  let (gChildren, oChildren) := match gp.kind, op.kind with
    | .tuple gElements, .tuple oElements => (gElements, oElements)
    | .constructor _ _ _ gFields, .constructor _ _ _ oFields => (gFields, oFields)
    | _, _ => (#[], #[])
  if gChildren.size == oChildren.size then
    for (gChild, oChild) in gChildren.zip oChildren do alignPattern g o gChild oChild

private partial def alignPlace (g o : ValidatedNamespace) (generated original : PlaceId) : AlignM Unit := do
  match g.places[generated.index]?, o.places[original.index]? with
  | some (.deref gBase), some (.deref oBase)
  | some (.field gBase _ _), some (.field oBase _ _)
  | some (.subslice gBase ..), some (.subslice oBase ..)
  | some (.unsize gBase), some (.unsize oBase)
  | some (.downcast gBase _), some (.downcast oBase _) => alignPlace g o gBase oBase
  | some (.index gBase gIndex), some (.index oBase oIndex) =>
      alignPlace g o gBase oBase
      alignExpr g o gIndex oIndex
  | _, _ => pure ()

/-- Conditions correspond by kind and by occurrence among that kind. -/
private partial def alignConditions (g o : ValidatedNamespace) (generated original : Array Condition) :
    AlignM Unit := do
  for h : index in [:generated.size] do
    let gCondition := generated[index]
    let occurrence := (generated.extract 0 index).countP (·.kind == gCondition.kind)
    let some oCondition := (original.filter (·.kind == gCondition.kind))[occurrence]?
      | continue
    pair gCondition.loc oCondition.loc
    alignExpr g o gCondition.expression oCondition.expression
    for (name, gAuxiliary) in gCondition.auxiliary do
      if let some (_, oAuxiliary) := oCondition.auxiliary.find? (·.1 == name) then
        alignExpr g o gAuxiliary oAuxiliary

end

/-- A rendering's temporaries collapse the blocks around its drops and calls,
where `alignExpr` stops. They run in the same order in both bodies, so they
align by it, the two bodies' sequences being of the same kinds. Aligned before
`alignExpr`, as of entries with equal generated ranges the first is taken. -/
private def alignDropsAndCalls (g o : ValidatedNamespace) (generated original : ExprId) :
    AlignM Unit := do
  let isDrop (expression : VerexIR.Expr) := expression.kind matches .operation (.drop _) ..
  let keep (_ : ExprId) (expression : VerexIR.Expr) :=
    isDrop expression || expression.kind matches .operation (.call (.function _)) ..
  let gSites := Verify.expressionsInOrder g keep generated #[]
  let oSites := Verify.expressionsInOrder o keep original #[]
  if gSites.map isDrop == oSites.map isDrop then
    for (gSite, oSite) in gSites.zip oSites do pair gSite.loc oSite.loc

private def alignContract (g o : ValidatedNamespace) (generated original : FunctionContract) :
    AlignM Unit := do
  if let (some gLoc, some oLoc) := (generated.loc, original.loc) then pair gLoc oLoc
  alignConditions g o generated.conditions original.conditions
  alignExprs g o generated.modifies original.modifies

/-- Declarations correspond by name. -/
private def counterpart? (g o : ValidatedNamespace) (declarations : Array α) (name : α → NameId)
    (generated : α) : Option α := do
  let key ← nameOf? g (name generated)
  declarations.find? fun candidate => nameOf? o (name candidate) == some key

private def alignNamespace (g o : ValidatedNamespace) : AlignM Unit := do
  pair g.loc o.loc
  for gDecl in g.constants do
    let some oDecl := counterpart? g o o.constants (·.name) gDecl | continue
    pair gDecl.loc oDecl.loc
    alignExpr g o gDecl.value oDecl.value
  for gDecl in g.structs do
    let some oDecl := counterpart? g o o.structs (·.name) gDecl | continue
    pair gDecl.loc oDecl.loc
    if gDecl.fields.size == oDecl.fields.size then
      for (gField, oField) in gDecl.fields.zip oDecl.fields do pair gField.loc oField.loc
    if gDecl.variants.size == oDecl.variants.size then
      for (gVariant, oVariant) in gDecl.variants.zip oDecl.variants do
        pair gVariant.loc oVariant.loc
    alignContract g o gDecl.contract oDecl.contract
  for gDecl in g.specFunctions do
    let some oDecl := counterpart? g o o.specFunctions (·.name) gDecl | continue
    pair gDecl.loc oDecl.loc
    alignOption g o gDecl.body oDecl.body
    alignContract g o gDecl.contract oDecl.contract
  for gDecl in g.lemmas do
    let some oDecl := counterpart? g o o.lemmas (·.name) gDecl | continue
    pair gDecl.loc oDecl.loc
    alignContract g o gDecl.contract oDecl.contract
    alignConditions g o gDecl.proof oDecl.proof
  if g.invariants.size == o.invariants.size then
    for (gDecl, oDecl) in g.invariants.zip o.invariants do
      pair gDecl.loc oDecl.loc
      alignConditions g o #[gDecl.condition] #[oDecl.condition]
  for gDecl in g.functions do
    let some oDecl := counterpart? g o o.functions (·.name) gDecl | continue
    pair gDecl.loc oDecl.loc
    alignContract g o gDecl.contract oDecl.contract
    if let (.structured gRoot, .structured oRoot) := (gDecl.body, oDecl.body) then
      alignDropsAndCalls g o gRoot oRoot
      alignExpr g o gRoot oRoot

/-! ## Source map -/

/-- A generated byte range, in the rendering `generated`, and the original range
it renders. -/
private structure Entry where
  generated : String
  start : Nat
  stop : Nat
  file : String
  origin : SourceRange

private def primary? (tables : Tables) (loc : LocId) : Option SourceRange :=
  tables.locations[loc.index]? |>.bind (·.primary)

private def sourceMap (generated original : ValidatedNamespace) : Array Entry :=
  let pairs := ((alignNamespace generated original).run #[]).2
  pairs.filterMap fun (gLoc, oLoc) => do
    let gRange ← primary? generated.localTables gLoc
    let oRange ← primary? original.localTables oLoc
    let file ← original.localTables.files[oRange.file.index]?
    let generatedFile ← generated.localTables.files[gRange.file.index]?
    pure { generated := generatedFile.name, start := gRange.startByte, stop := gRange.endByte
           file := file.name, origin := oRange }

/-- The innermost entry around a range of a rendering. -/
private def innermost? (entries : Array Entry) (generated : String) (start stop : Nat) :
    Option Entry :=
  entries.foldl (init := none) fun best entry =>
    if entry.generated == generated && entry.start ≤ start && stop ≤ entry.stop then
      match best with
      | some current => if entry.stop - entry.start < current.stop - current.start
          then some entry else some current
      | none => some entry
    else best

/-! ## Driver -/

/-- Read source files once each, as maps from bytes to positions. -/
private def fileMap (cache : IO.Ref (Std.HashMap String (Option FileMap))) (file : String) :
    IO (Option FileMap) := do
  if let some cached := (← cache.get)[file]? then return cached
  let loaded ← try some <$> FileMap.ofString <$> IO.FS.readFile file catch _ => pure none
  cache.modify (·.insert file loaded)
  pure loaded

/-- A companion's items, indented by `indent` into their namespace and
scope, occupying the lines `firstLine` to `lastLine` of the generated file. -/
private structure Splice where
  path : System.FilePath
  firstLine : Nat
  lastLine : Nat
  indent : Nat
  generated : Bool := false

private def spliceIndent : Nat := 2

private def indentItems (indent : Nat) (text : String) : String :=
  "\n".intercalate <| (text.splitOn "\n").map fun line =>
    if line.trimAscii.isEmpty then "" else "".pushn ' ' indent ++ line

/-- The sources of a unit, each the path its namespaces' tables name. -/
private def sourceFiles (unit : ValidatedUnit) : Array String :=
  unit.namespaces.foldl (init := #[]) fun found ns =>
    ns.localTables.files.foldl (init := found) fun found file =>
      if found.contains file.name then found else found.push file.name

/-- The `verex sources at` command recording the sources a rendering is made
from, relative to `directory`, with the hashes of their content. -/
private def sourcesCommand (unit : ValidatedUnit) (directory : System.FilePath) : IO String := do
  let mut lines := #[s!"verex sources at {(toString directory).quote}"]
  for file in sourceFiles unit do
    let hash ← try some <$> VerexSL.Proofs.contentHash (directory / file) catch _ => pure none
    if let some hash := hash then lines := lines.push s!"  {file.quote} {hash}"
  return "\n".intercalate lines.toList ++ "\n"

/-- The generated file: the prelude, each namespace followed by the items of
its spliced companions, with every splice's lines recorded, and last the
record of its sources, which leaves the positions before it as they are
wherever the rendering is made. -/
private def compose (prelude sources : String) (order : Array NamespaceId)
    (namespaces : Array String) (companions : Array Companion) : String × Array Splice :=
  Id.run do
    let mut generated := prelude
    let mut splices := #[]
    for identity in order, rendered in namespaces do
      while !generated.endsWith "\n\n" do generated := generated ++ "\n"
      generated := generated ++ rendered ++ "\n"
      for companion in companions do
        if companion.namespaceId != identity || companion.role != .spliced then continue
        let some text := companion.text | continue
        -- A module file's items reopen its scope, one `mod` per segment.
        let indent := spliceIndent * (companion.scope.size + 1)
        let items := (indentItems indent text).trimAsciiEnd.toString
        generated := generated ++ "\n"
        for (segment, depth) in companion.scope.zipIdx do
          generated := generated ++ "".pushn ' ' (spliceIndent * (depth + 1)) ++
            s!"mod {Print.sourceIdentifier segment} where\n"
        let firstLine := (generated.splitOn "\n").length
        generated := generated ++ items ++ "\n"
        let lastLine := firstLine + (items.splitOn "\n").length - 1
        splices := splices.push {
          path := companion.path, firstLine, lastLine, indent, generated := companion.generated }
    while !generated.endsWith "\n\n" do generated := generated ++ "\n"
    (generated ++ sources, splices)

/-- Import `modules` into this process, whose executable links them, so
their elaborators run natively. Compiled namespaces are found under `roots`. -/
unsafe def importModulesUnsafe (modules : Array Name) (roots : Array System.FilePath := #[]) :
    IO Environment := do
  enableInitializersExecution
  initSearchPath (← findSysroot)
  searchPathRef.modify (roots.toList ++ ·)
  importModules (modules.map ({ module := · })) {} (trustLevel := 1024) (loadExts := true)

@[implemented_by importModulesUnsafe]
opaque importLibraries (modules : Array Name) (roots : Array System.FilePath := #[]) :
    IO Environment

/-- Import `VerexSL` into this process. -/
def importVerexSL : IO Environment := importLibraries #[`VerexSL]

/-- An elaborated command: its syntax, its messages, and the state after it. -/
private structure Elaborated where
  stx : Syntax
  messages : Array Message
  state : Elab.Command.State

/-- Elaborate the commands of an input after its header, as the language
server does, each with its messages and the state after it. -/
private partial def elaborateCommands (inputCtx : Parser.InputContext)
    (parserState : Parser.ModuleParserState) (commandState : Elab.Command.State) :
    IO (Array Elaborated) := do
  let first ← Language.Lean.processCommands inputCtx parserState commandState
  let rec walk (snapshot : Language.Lean.CommandParsedSnapshot) (found : Array Elaborated) :
      Array Elaborated :=
    let messages := (Language.toSnapshotTree snapshot.elabSnap).getAll.foldl
      (fun log node => log ++ node.diagnostics.msgLog) snapshot.diagnostics.msgLog
    let found := found.push { stx := snapshot.stx, messages := messages.toArray
                              state := snapshot.elabSnap.resultSnap.get.cmdState }
    match snapshot.nextCmdSnap? with
    | some next => walk next.task.get found
    | none => found
  return walk first.get #[]

/-- The options both the rendering and its proof files are elaborated with. -/
private def driverOptions (options : Options) : Options :=
  -- The driver verifies, as the command line does; only the language server
  -- skips by default. It reports every message in its own file.
  let options := if (options.find? `verex.skip_verify).isSome then options
    else options.setBool `verex.skip_verify false
  verex.relocateMessages.set options false

/-- The root of the Lean modules of compiled namespaces. -/
def namespaceModuleRoot : Name := `VerexNamespaces

/-- The Lean module of the compiled namespace at a path. -/
def namespaceModule (path : Array String) : Name :=
  path.foldl Name.str namespaceModuleRoot

/-- The file of a namespace's Lean module under a root of compiled namespaces. -/
def namespaceFile (root : System.FilePath) (module : Name) (extension : String) :
    System.FilePath :=
  (module.components.foldl (fun file part => file / part.toString (escape := false)) root).addExtension
    extension

/-- Elaborate a rendering whose header imports `VerexSL` and compiled namespaces,
over an environment that has imported them: its commands. -/
private def elaborateRendering (environment : Environment) (input fileName : String)
    (options : Options) : IO (Array Message × Array Elaborated) := do
  let inputCtx := Parser.mkInputContext input fileName
  let (header, parserState, messages) ← Parser.parseHeader inputCtx
  let imports := (Elab.headerToImports header (includeInit := false)).map (·.module)
  unless imports[0]? == some `VerexSL &&
      (imports.extract 1).all (·.getRoot == namespaceModuleRoot) do
    throw <| IO.userError "a VerexSL rendering imports `VerexSL` and compiled namespaces"
  for module in imports do
    if (environment.getModuleIdx? module).isNone then
      throw <| IO.userError s!"a VerexSL rendering imports `{module}`, which is not loaded"
  let commands ← elaborateCommands inputCtx parserState
    (Elab.Command.mkState environment {} (driverOptions options))
  pure (messages.toArray, commands)

/-- The options `VEREX_OPTIONS` sets beside a request's, for debugging a
verification the CLI or the benchmark runs: `name=value` assignments
separated by commas, each value read at the type its registered option
declares, as `lean -D` reads one. -/
def environmentOptions (options : Options) : IO Options := do
  let some assignments ← IO.getEnv "VEREX_OPTIONS" | return options
  let declarations ← getOptionDecls
  (assignments.splitOn ",").foldlM (init := options) fun options assignment => do
    let [name, value] := assignment.splitOn "="
      | throw <| IO.userError s!"VEREX_OPTIONS: `{assignment}` is not `name=value`"
    let name := name.toName
    let some declaration := declarations.find? name
      | throw <| IO.userError s!"VEREX_OPTIONS: `{name}` is not a registered option"
    match declaration.defValue with
    | .ofBool _ => match value with
      | "true" => return options.setBool name true
      | "false" => return options.setBool name false
      | _ => throw <| IO.userError s!"VEREX_OPTIONS: `{name}` takes `true` or `false`"
    | .ofNat _ => match value.toNat? with
      | some number => return options.set name number
      | none => throw <| IO.userError s!"VEREX_OPTIONS: `{name}` takes a numeral"
    | .ofString _ => return options.set name value
    | _ => throw <| IO.userError s!"VEREX_OPTIONS: `{name}` is not a Bool, Nat, or String option"

/-- Elaborate a rendering from a file over an environment that has imported
`VerexSL`, natively, the way verifying a source elaborates its own, for what
`scripts/isolate-target.py` cuts: its messages. -/
def elaborateFile (environment : Environment) (path : System.FilePath) : IO (Array Message) := do
  let (messages, commands) ← elaborateRendering environment (← IO.FS.readFile path) path.toString
    (← environmentOptions {})
  return commands.foldl (init := messages) (· ++ ·.messages)

/-- Elaborate a rendering over compiled namespaces: imports `libraries` and
the rendering's own imports, the namespaces found under `root`. -/
def elaborateFileOver (libraries : Array Name) (path root : System.FilePath) :
    IO (Array Message) := do
  let (header, _, _) ← Parser.parseHeader (Parser.mkInputContext (← IO.FS.readFile path) path.toString)
  let imports := (Elab.headerToImports header (includeInit := false)).map (·.module)
  elaborateFile (← importLibraries (libraries ++ imports) #[root]) path

/-- Elaborate a proof file over the state its module's command left: its
messages. Its imports must be loaded already, and it holds its header and
`verex proofs where`, which verifies the module. -/
private def elaborateProofFile (state : Elab.Command.State) (companion : Companion) (text : String)
    (module : Name) (options : Options) : IO (Array Message) := do
  let fileName := companion.path.toString
  let inputCtx := Parser.mkInputContext text fileName
  let (header, parserState, messages) ← Parser.parseHeader inputCtx
  let atStart (data : MessageData) : Message :=
    { fileName, pos := ⟨1, 0⟩, severity := .error, data }
  for import_ in Elab.headerToImports header (includeInit := false) do
    if (state.env.getModuleIdx? import_.module).isNone then
      return messages.toArray.push <| atStart m!"a proof file imports only what the \
        verification of its source loads, which `{import_.module}` is not"
  let commands ← elaborateCommands inputCtx parserState
    (Elab.Command.mkState state.env {} (driverOptions options))
  let messages := commands.foldl (init := messages.toArray) (· ++ ·.messages)
  let items := commands.filter (!Parser.isTerminalCommand ·.stx)
  unless items.size == 2 && (items[1]?.any (·.stx.isOfKind ``VerexSL.Proofs.verexProofsCommand)) do
    return messages.push <| atStart m!"a proof file holds its header, then `verex proofs where` \
      and the items it verifies"
  -- `verex proofs where` verifies the module, or reports why it does not.
  let verified := commands.back?.any fun last =>
    ((VerexSL.Verify.deferredModules last.state.env).find? module).any (·.verified)
  unless verified || messages.any (·.severity == .error) do
    return messages.push <| atStart m!"`verex proofs where` does not verify `{module}`"
  return messages

/-- The options a request's fields set, beside those of `VEREX_OPTIONS`. -/
private def verificationOptions (heartbeats jobs : Option Nat) (ignoreProofs : Bool) :
    IO Options := do
  let options := match heartbeats with
    | some heartbeats => verex.verifyHeartbeats.set {} (heartbeats * 1000)
    | none => {}
  let options := if ignoreProofs then verex.ignoreProofs.set options true else options
  let options := match jobs with
    | some jobs => verex.jobs.set options jobs
    | none => options
  environmentOptions options

/-- The namespaces a namespace's companions name, which it follows. -/
private def companionUses (companions : Array Companion) (identity : NamespaceId) :
    Array NamespaceId :=
  companions.flatMap fun companion =>
    if companion.namespaceId == identity then companion.uses else #[]

/-- A namespace rendered to a file of its own for `compileNamespace`. -/
structure RenderedNamespace where
  path : Array String
  module : Name
  file : System.FilePath
  /-- The modules of the namespaces it uses. -/
  imports : Array Name

/-- Render every namespace of a unit to a file of its own under `root`, importing
the namespaces it uses. -/
def renderNamespaces (environment : Environment) (unit : ValidatedUnit)
    (companions : Array Companion) (root : System.FilePath) :
    IO (Except Print.Error (Array RenderedNamespace)) := do
  let spliced := companionUses companions
  let order ← match Print.dependencyOrder unit spliced with
    | .ok order => pure order
    | .error message => throw <| IO.userError message
  let moduleOf (identity : NamespaceId) : Name :=
    namespaceModule ((unit.tables.namespaces[identity.index]?.map (·.segments)).getD #[])
  let mut rendered := #[]
  for identity in order do
    let some ns := unit.namespaces[identity.index]? | continue
    let imports := (Print.namespaceUsesIn unit spliced ns).map moduleOf
    match Print.renderNamespaceParts environment unit #[identity] (imports := imports) with
    | .error error => return .error error
    | .ok (prelude, parts) =>
        let module := moduleOf identity
        let file := namespaceFile root module "lean"
        if let some parent := file.parent then IO.FS.createDirAll parent
        IO.FS.writeFile file (compose prelude "" #[identity] parts companions).1
        let path := (unit.tables.namespaces[identity.index]?.map (·.segments)).getD #[]
        rendered := rendered.push { path, module, file, imports }
  return .ok rendered

/-- Elaborate a rendered namespace over `libraries` and the namespaces its header
imports, and write its `.olean` beside it unless an error is among its messages,
which it returns. -/
def compileNamespace (libraries : Array Name) (root : System.FilePath) (module : Name)
    (heartbeats : Option Nat := none) (ignoreProofs : Bool := false) : IO (Array Message) := do
  let file := namespaceFile root module "lean"
  let input ← IO.FS.readFile file
  let inputCtx := Parser.mkInputContext input file.toString
  let (header, _, _) ← Parser.parseHeader inputCtx
  let imports := (Elab.headerToImports header (includeInit := false)).map (·.module)
  let environment ← importLibraries (libraries ++ imports.filter (· != `VerexSL)) #[root]
  let environment := environment.setMainModule module
  let (messages, commands) ← elaborateRendering environment input file.toString
    (← verificationOptions heartbeats none ignoreProofs)
  let messages := commands.foldl (init := messages) (· ++ ·.messages)
  unless messages.any (·.severity == .error) do
    let elaborated := commands.back?.map (·.state.env) |>.getD environment
    let olean := namespaceFile root module "olean"
    let partial_ := olean.addExtension "partial"
    writeModule elaborated partial_
    IO.FS.rename partial_ olean
  return messages

/-- What a rendering over compiled namespaces imports, and what it elaborates: the
namespaces before its target are imported; it holds the target and what follows,
as far as the last namespace that verifies a function of the target against its
invariants, each registered as where all are elaborated. -/
private def compiledParts (unit : ValidatedUnit) (order : Array NamespaceId)
    (compiled : CompiledNamespaces) : Array Name × Array NamespaceId :=
  let position := order.idxOf compiled.target
  let writers := match unit.namespaces[compiled.target.index]? with
    | some ns => ns.functions
    | none => #[]
  let target? := unit.namespaces[compiled.target.index]?
  let owes (identity : NamespaceId) : Bool :=
    match target?, unit.namespaces[identity.index]? with
    | some target, some ns =>
        !ns.invariants.isEmpty && writers.any
          (Verify.owesWriterTarget unit identity compiled.target target)
    | _, _ => false
  let stop := (order.zipIdx.foldl (init := position + 1) fun stop (identity, index) =>
    if index > position && owes identity then index + 1 else stop)
  ((order.extract 0 position).map fun identity =>
      namespaceModule ((unit.tables.namespaces[identity.index]?.map (·.segments)).getD #[]),
    order.extract position stop)

/-- Import what verifying a request's target over compiled namespaces elaborates:
`libraries` and the namespaces before the target. -/
def importCompiled (libraries : Array Name) (request : Request) (compiled : CompiledNamespaces) :
    IO Environment := do
  let order ← match Print.dependencyOrder request.unit (companionUses request.companions) with
    | .ok order => pure order
    | .error message => throw <| IO.userError message
  importLibraries (libraries ++ (compiledParts request.unit order compiled).1) #[compiled.root]

/-- Render, write, and elaborate a unit, and report every message in the
coordinates of its source. The environment has imported `VerexSL` and what
the proof files import, and, with `compiled`, the namespaces before the target
(`importCompiled`). -/
def run (environment : Environment) (request : Request) : IO (Array Report) := do
  let cache ← IO.mkRef ({} : Std.HashMap String (Option FileMap))
  let root ← match request.sourceRoot with
    | some root => pure root
    | none => IO.currentDir
  -- A source name as reported: under the source root, when one is named,
  -- else relative to the directory the command runs in.
  let display (file : String) : String :=
    if request.sourceRoot.isSome then (root / file).toString
    else (file.dropPrefix s!"{root}/").toString
  let rendered ← Perf.withPhase .render do
    let order ← match Print.dependencyOrder request.unit (companionUses request.companions) with
      | .ok order => pure order
      | .error message => throw <| IO.userError message
    let (imports, order) := match request.compiled with
      | some compiled => compiledParts request.unit order compiled
      | none => (#[], order)
    match Print.renderNamespaceParts environment request.unit order (imports := imports) with
    | .ok (prelude, namespaces) =>
        let (generated, splices) :=
          compose prelude (← sourcesCommand request.unit root) order namespaces request.companions
        if let some parent := request.output.parent then IO.FS.createDirAll parent
        IO.FS.writeFile request.output generated
        pure (Except.ok (generated, splices))
    | .error error => pure (Except.error error)
  let (generated, splices) ← match rendered with
    | .ok rendered => pure rendered
    | .error error =>
        let report ← match error.location with
          | some (file, range) => do
              let position := (← fileMap cache (root / file.name).toString).map
                (·.toPosition ⟨range.startByte⟩)
              pure { file := display file.name, line := position.map (·.line) |>.getD 1
                     column := position.map (·.column + 1) |>.getD 1
                     severity := .error, text := error.message }
          | none => pure { file := request.output.toString, line := 1, column := 1
                           severity := .error, text := error.message }
        return #[report]
  if request.renderOnly then return #[]
  let generatedName := request.output.toString
  let options ← verificationOptions request.heartbeats request.jobs request.ignoreProofs
  -- A namespace with a proof file is verified by it.
  let proofFiles := request.companions.filter fun companion =>
    companion.role == .proofs && companion.text.isSome
  let environment := VerexSL.Verify.setDeferral environment
    (.proofFiles (proofFiles.map (·.path.toString)))
  let environment := VerexSL.Verify.setRenderedSource environment { unit := request.unit, display }
  let (headerMessages, commands) ← elaborateRendering environment generated generatedName options
  let mut messages := headerMessages
  let mut deferred : NameSet := {}
  for command in commands do
    messages := messages ++ command.messages
    for (name, module) in (VerexSL.Verify.deferredModules command.state.env).toList do
      if deferred.contains name then continue
      deferred := deferred.insert name
      let some companion := proofFiles.find? (some ·.path.toString == module.proofFile) | continue
      let some text := companion.text | continue
      messages := messages ++ (← elaborateProofFile command.state companion text name options)
  let elaborated := commands.back?.map (·.state.env) |>.getD environment
  -- Each elaborated namespace aligns with the source namespace at its path.
  let entries := (moduleUnits elaborated).toArray.flatMap fun (_, unit) =>
    unit.namespaces.flatMap fun generatedNs =>
      match request.unit.namespaces.find? (Verify.pathKey · == Verify.pathKey generatedNs) with
      | some original => sourceMap generatedNs original
      | none => #[]
  let generatedMap := FileMap.ofString generated
  let mut reports := #[]
  -- A refused function is an error of the crate at what refused it, however the
  -- others verify.
  for ns in request.unit.namespaces do
    for function in ns.functions do
      let some refused := function.profileData.find? (·.tag == VerexIR.rustRefusedTag) | continue
      let some location := ns.location? function.loc | continue
      let some range := location.related.back? <|> location.primary | continue
      let some file := ns.localTables.files[range.file.index]? | continue
      let position := (← fileMap cache (root / file.name).toString).map
        (·.toPosition ⟨range.startByte⟩)
      let name := ((ns.nameOf function.name).map (·.name)).getD "?"
      reports := reports.push {
        file := display file.name, line := position.map (·.line) |>.getD 1
        column := position.map (·.column + 1) |>.getD 1, severity := .error
        text := s!"`{name}` is refused, so it is not verified and a call of it needs a \
          contract: {refused.payload}" }
  for message in messages do
    if message.isSilent then continue
    let text ← message.data.toString
    -- A message in a file no entry maps, a proof file, is at its own position.
    unless message.fileName == generatedName || entries.any (·.generated == message.fileName) do
      reports := reports.push {
        file := message.fileName, line := message.pos.line, column := message.pos.column + 1
        severity := message.severity, text }
      continue
    let spliced := if message.fileName != generatedName then none else
      splices.find? fun splice =>
        splice.firstLine ≤ message.pos.line && message.pos.line ≤ splice.lastLine
    let report ← match spliced with
      | some splice => pure {
          file := if splice.generated then generatedName else splice.path.toString
          line := if splice.generated then message.pos.line
            else message.pos.line - splice.firstLine + 1
          column := if splice.generated then message.pos.column + 1
            else message.pos.column - splice.indent + 1
          severity := message.severity, text }
      | none => do
          let fileMapOf ← if message.fileName == generatedName then pure generatedMap else
            pure ((← fileMap cache message.fileName).getD generatedMap)
          let start := (fileMapOf.ofPosition message.pos).byteIdx
          let stop := (message.endPos.map fileMapOf.ofPosition).map (·.byteIdx) |>.getD start
          let origin ← match innermost? entries message.fileName start stop with
            | some entry => pure <| (← fileMap cache (root / entry.file).toString).map fun map =>
                (display entry.file, map.toPosition ⟨entry.origin.startByte⟩)
            | none => pure none
          pure <| match origin with
            | some (file, position) => {
                file, line := position.line, column := position.column + 1
                severity := message.severity, text }
            | none => {
                file := message.fileName, line := message.pos.line
                column := message.pos.column + 1, severity := message.severity, text }
    reports := reports.push report
  pure reports

end VerexSL.SourceVerify
