-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerLang

/-!
# Verifying a source through its LeanerLang rendering

A frontend's validated unit is rendered as LeanerLang, written out for
inspection, and elaborated in this process, which verifies every specified
function. LeanerLang items authored beside a source — a Rust file's
specifications in its `.spec.lean`, a Move file's proofs in its `.proof.lean`
— are spliced into the rendered namespace of that source, whose `proof_file`
pragma names the file.

Every message is reported in the coordinates of the file it came from. A
position in the spliced items maps line by line to their file.
Any other position maps through the correspondence between the source's unit
and the unit elaborated from the rendering: the two are aligned declaration by
declaration and then node by node while their shapes agree, and a position
takes the source range of the innermost aligned node around it. A position
outside every aligned node stays in the generated file.
-/

namespace LeanerLang.SourceVerify

open Lean LeanerIR LeanerIR.Validation

/-- The LeanerLang items authored beside a source, spliced into the
namespace rendered from it: its specifications (Rust) or proofs (Move). The
frontend names the file on the namespace's `proof_file` pragma whether or
not it exists, so a message asking for a proof can say where to write it. -/
structure Companion where
  path : System.FilePath
  namespaceId : NamespaceId
  /-- The file's items; `none` when the file does not exist. -/
  text : Option String := none

structure Request where
  unit : ValidatedUnit
  companions : Array Companion := #[]
  /-- Where the generated LeanerLang is written. -/
  output : System.FilePath
  /-- Write the rendering only, without elaborating it: for inspection and
  for tools that elaborate its parts themselves. -/
  renderOnly : Bool := false
  /-- The heartbeat budget of a function's verification where no `pragma
  heartbeats` sets it, in thousands of `maxHeartbeats` units; the default of
  `leaner.verifyHeartbeats` without one. -/
  heartbeats : Option Nat := none

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

private def nameOf? (ns : ValidatedNamespace) (id : NameId) : Option String :=
  ns.tables.names[id.index]?.map (·.name)

mutual

private partial def alignExpr (g o : ValidatedNamespace) (generated original : ExprId) : AlignM Unit := do
  let some ge := g.expressions[generated.index]? | return
  let some oe := o.expressions[original.index]? | return
  pair ge.loc oe.loc
  match ge.kind, oe.kind with
  | .operation _ _ gArgs _, .operation _ _ oArgs _ => alignExprs g o gArgs oArgs
  | .block gStatements gResult, .block oStatements oResult =>
      alignExprs g o gStatements oStatements
      alignOption g o gResult oResult
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
  if g.invariants.size == o.invariants.size then
    for (gDecl, oDecl) in g.invariants.zip o.invariants do
      pair gDecl.loc oDecl.loc
      alignConditions g o #[gDecl.condition] #[oDecl.condition]
  for gDecl in g.functions do
    let some oDecl := counterpart? g o o.functions (·.name) gDecl | continue
    pair gDecl.loc oDecl.loc
    alignContract g o gDecl.contract oDecl.contract
    if let (.structured gRoot, .structured oRoot) := (gDecl.body, oDecl.body) then
      alignExpr g o gRoot oRoot

/-! ## Source map -/

/-- A generated byte range and the original range it renders. -/
private structure Entry where
  start : Nat
  stop : Nat
  file : String
  origin : SourceRange

private def primary? (tables : Tables) (loc : LocId) : Option SourceRange :=
  tables.locations[loc.index]? |>.bind (·.primary)

private def sourceMap (generated original : ValidatedNamespace) : Array Entry :=
  let pairs := ((alignNamespace generated original).run #[]).2
  pairs.filterMap fun (gLoc, oLoc) => do
    let gRange ← primary? generated.tables gLoc
    let oRange ← primary? original.tables oLoc
    let file ← original.tables.files[oRange.file.index]?
    pure { start := gRange.startByte, stop := gRange.endByte, file := file.name, origin := oRange }

/-- The innermost entry around a generated range. -/
private def innermost? (entries : Array Entry) (start stop : Nat) : Option Entry :=
  entries.foldl (init := none) fun best entry =>
    if entry.start ≤ start && stop ≤ entry.stop then
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

/-- A companion's items, indented into their namespace, occupying the lines
`firstLine` to `lastLine` of the generated file. -/
private structure Splice where
  path : System.FilePath
  firstLine : Nat
  lastLine : Nat

private def spliceIndent : Nat := 2

private def indentItems (text : String) : String :=
  "\n".intercalate <| (text.splitOn "\n").map fun line =>
    if line.trimAscii.isEmpty then "" else "".pushn ' ' spliceIndent ++ line

/-- The generated file: the prelude, then each namespace followed by the
items of its companions, with every splice's lines recorded. -/
private def compose (prelude : String) (order : Array NamespaceId)
    (namespaces : Array String) (companions : Array Companion) : String × Array Splice :=
  Id.run do
    let mut generated := prelude
    let mut splices := #[]
    for identity in order, rendered in namespaces do
      while !generated.endsWith "\n\n" do generated := generated ++ "\n"
      generated := generated ++ rendered ++ "\n"
      for companion in companions do
        if companion.namespaceId != identity then continue
        let some text := companion.text | continue
        let items := (indentItems text).trimAsciiEnd.toString
        generated := generated ++ "\n"
        let firstLine := (generated.splitOn "\n").length
        generated := generated ++ items ++ "\n"
        let lastLine := firstLine + (items.splitOn "\n").length - 1
        splices := splices.push { path := companion.path, firstLine, lastLine }
    (generated, splices)

/-- Import `LeanerLang` into this process. The executable links the library,
so its elaborators run natively. -/
unsafe def importLeanerLangUnsafe : IO Environment := do
  enableInitializersExecution
  initSearchPath (← findSysroot)
  importModules #[{ module := `LeanerLang }] {} (trustLevel := 1024) (loadExts := true)

@[implemented_by importLeanerLangUnsafe]
opaque importLeanerLang : IO Environment

/-- Elaborate a rendering whose header imports exactly `LeanerLang`, over an
environment that has imported it. -/
private def elaborate (environment : Environment) (input fileName : String)
    (options : Options) : IO (Environment × MessageLog) := do
  let inputCtx := Parser.mkInputContext input fileName
  let (header, parserState, messages) ← Parser.parseHeader inputCtx
  unless (Elab.headerToImports header (includeInit := false)).map (·.module) == #[`LeanerLang] do
    throw <| IO.userError "a LeanerLang rendering imports exactly `LeanerLang`"
  let state ← Elab.IO.processCommands inputCtx parserState
    (Elab.Command.mkState environment messages options)
  pure (state.commandState.env, state.commandState.messages)

/-- The unit's owned namespaces, each after the owned namespaces it uses. -/
private def dependencyOrder (unit : ValidatedUnit) : Except String (Array NamespaceId) := do
  let owned := unit.namespaces.map (·.identity)
  let uses (ns : ValidatedNamespace) :=
    (Print.referencedNamespaces unit ns).filter owned.contains
  let mut ordered : Array NamespaceId := #[]
  -- Each round places every namespace whose uses are placed; a round that
  -- places none leaves a cycle.
  while ordered.size < owned.size do
    let ready := unit.namespaces.filter fun ns =>
      !ordered.contains ns.identity && (uses ns).all ordered.contains
    if ready.isEmpty then throw "the unit's modules use each other in a cycle"
    ordered := ordered ++ ready.map (·.identity)
  return ordered

/-- The namespace path a validated namespace is declared at, without a
Move address alias, which a rendering does not spell. -/
private def pathKey (ns : ValidatedNamespace) : Option (String × String) := do
  let segments ← ns.tables.namespaces[ns.identity.index]? |>.map (·.segments)
  return (← segments[0]?, ← segments.back?)

/-- Render, write, and elaborate a unit, and report every message in the
coordinates of its source. -/
def run (environment : Environment) (request : Request) : IO (Array Report) := do
  let cache ← IO.mkRef ({} : Std.HashMap String (Option FileMap))
  let rendered ← Perf.withPhase .render do
    let order ← match dependencyOrder request.unit with
      | .ok order => pure order
      | .error message => throw <| IO.userError message
    match Print.renderNamespaceParts environment request.unit order with
    | .ok (prelude, namespaces) =>
        let (generated, splices) := compose prelude order namespaces request.companions
        IO.FS.writeFile request.output generated
        pure (Except.ok (generated, splices))
    | .error error => pure (Except.error error)
  let (generated, splices) ← match rendered with
    | .ok rendered => pure rendered
    | .error error =>
        let report ← match error.location with
          | some (file, range) => do
              let position := (← fileMap cache file.name).map (·.toPosition ⟨range.startByte⟩)
              pure { file := file.name, line := position.map (·.line) |>.getD 1
                     column := position.map (·.column + 1) |>.getD 1
                     severity := .error, text := error.message }
          | none => pure { file := request.output.toString, line := 1, column := 1
                           severity := .error, text := error.message }
        return #[report]
  if request.renderOnly then return #[]
  let generatedName := request.output.toString
  let options := match request.heartbeats with
    | some heartbeats => leaner.verifyHeartbeats.set {} (heartbeats * 1000)
    | none => {}
  let (elaborated, messages) ← elaborate environment generated generatedName options
  -- Each elaborated namespace aligns with the source namespace at its path.
  let entries := (moduleUnits elaborated).toArray.flatMap fun (_, unit) =>
    unit.namespaces.flatMap fun generatedNs =>
      match request.unit.namespaces.find? (pathKey · == pathKey generatedNs) with
      | some original => sourceMap generatedNs original
      | none => #[]
  let generatedMap := FileMap.ofString generated
  let mut reports := #[]
  for message in messages.toList do
    if message.isSilent then continue
    let text ← message.data.toString
    let spliced := splices.find? fun splice =>
      splice.firstLine ≤ message.pos.line && message.pos.line ≤ splice.lastLine
    let report ← match spliced with
      | some splice => pure {
          file := splice.path.toString
          line := message.pos.line - splice.firstLine + 1
          column := message.pos.column - spliceIndent + 1
          severity := message.severity, text }
      | none => do
          let start := (generatedMap.ofPosition message.pos).byteIdx
          let stop := (message.endPos.map generatedMap.ofPosition).map (·.byteIdx) |>.getD start
          let origin ← match innermost? entries start stop with
            | some entry => pure <| (← fileMap cache entry.file).map fun map =>
                (entry.file, map.toPosition ⟨entry.origin.startByte⟩)
            | none => pure none
          pure <| match origin with
            | some (file, position) => {
                file, line := position.line, column := position.column + 1
                severity := message.severity, text }
            | none => {
                file := generatedName, line := message.pos.line
                column := message.pos.column + 1, severity := message.severity, text }
    reports := reports.push report
  pure reports

end LeanerLang.SourceVerify
