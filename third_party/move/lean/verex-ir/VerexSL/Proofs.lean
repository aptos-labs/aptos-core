-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify

/-!
# Proof files

The proofs of a module rendered from a source live in a proof file beside
the source, a Lean file whose header establishes the module's context:

```lean
import VerexMove.Proofs
verex proofs for "pool_u64.move"
verex proofs where

verify buy_in by lean =>
  …
```

A frontend's header command, Move's `verex proofs for`, selects the module.
Where the source-verification driver elaborates the proof file, the module is
registered already, its verification deferred to the file. Elsewhere, as in
the editor, the header loads it from the rendering of its source the driver
last wrote: the module and the modules before it are registered, their
verification deferred. `verex proofs where` then verifies the module with
its proof file's items, as its own command verifies a module with its items.
-/

namespace VerexSL.Proofs

open Lean Elab Command
open VerexSL.Verify

/-- One source of a rendering: its path and the hash of its content. -/
syntax verexRenderingSource := str num

/-- The sources a rendering was made from, relative to the directory it was
made in, each with the hash of its content then: what a proof file that loads
the rendering checks it against. The rendering's last command. -/
syntax (name := verexSourcesCommand)
  "verex" &"sources" "at" str (ppLine verexRenderingSource)* : command

@[command_elab verexSourcesCommand]
def elabSources : CommandElab := fun _ => pure ()

/-- The hash of a file's content a rendering records. -/
def contentHash (path : System.FilePath) : IO UInt64 :=
  return (← IO.FS.readBinFile path).hash

/-- `verex proofs where` and the items that follow: a proof file's `verify`
items, theorems, and `open` items. -/
syntax (name := verexProofsCommand)
  "verex" &"proofs" "where" ppLine manyIndent(verexItem) : command

private def realPath? (path : System.FilePath) : IO (Option System.FilePath) :=
  try some <$> IO.FS.realPath path catch _ => pure none

private def select (name : Name) : CommandElabM Unit :=
  modifyEnv (selectProofModule · name)

/-- The deferred module whose `proof_file` pragma, read relative to
`directory`, names `proofFile`. -/
private def deferredFor? (directory proofFile : System.FilePath) :
    CommandElabM (Option DeferredModule) := do
  let some target ← realPath? proofFile | return none
  for (_, module) in (deferredModules (← getEnv)).toList do
    let some named := module.proofFile | continue
    if (← realPath? (directory / named)) == some target then return some module
  return none

/-- Select the module registered with its verification deferred whose proof
file is `proofFile`, as where the driver elaborates the file: whether one is.
Its `proof_file` pragma is relative to the directory the driver runs in. -/
def selectRegistered (proofFile : System.FilePath) : CommandElabM Bool := do
  let some module ← deferredFor? "." proofFile | return false
  select module.name
  return true

/-- The `proof_file` pragma of a module command. -/
private def proofFilePragma? (command : Syntax) : Option String := do
  guard <| command.isOfKind ``verexMoveModuleCommand ||
    command.isOfKind ``verexNamespaceCommand || command.isOfKind ``verexRustNamespaceCommand
  let items := command[command.getNumArgs - 1].getArgs
  items.findSome? fun item => do
    guard <| item.isOfKind ``verexNamespacePragmaItem
    let name ← item.find? (·.isIdent)
    guard <| name.getId.toString (escape := false) == "proof_file"
    (item.find? (·.isStrLit?.isSome)) >>= (·.isStrLit?)

/-- The directory and sources a rendering's `verex sources at` records. -/
private def sourcesOf? (command : Syntax) : Option (String × Array (String × Nat)) := do
  guard <| command.isOfKind ``verexSourcesCommand
  let directory ← command[3].isStrLit?
  let sources := command[4].getArgs.filterMap fun entry => do
    pure (← entry[0].isStrLit?, ← entry[1].isNatLit?)
  pure (directory, sources)

/-- Load the context of the proof file `proofFile` from `rendering`: the module
whose `proof_file` pragma names the file, and the modules rendered before it,
are registered with their verification deferred, and the module is selected.
Messages of the rendering's commands are reported at `ref`. Whether the
rendering renders the module; one that does not is left unloaded. -/
def loadRendering (ref : Syntax) (rendering proofFile : System.FilePath) :
    CommandElabM Bool := do
  let text ← IO.FS.readFile rendering
  let inputCtx := Parser.mkInputContext text rendering.toString
  let (header, parserState, log) ← Parser.parseHeader inputCtx
  unless (Elab.headerToImports header (includeInit := false)).map (·.module) == #[`VerexSL] do
    throwErrorAt ref m!"`{rendering}` is not a rendering: a rendering imports exactly `VerexSL`"
  let parsing := { env := ← getEnv, options := ← getOptions }
  let mut state := parserState
  let mut log := log
  let mut commands := #[]
  repeat
    let (command, next, logged) := Parser.parseCommand inputCtx parsing state log
    state := next
    log := logged
    if Parser.isTerminalCommand command then break
    commands := commands.push command
  let some (directory, sources) := commands.findSome? sourcesOf?
    | throwErrorAt ref m!"`{rendering}` records no sources; verify its source again"
  let directory : System.FilePath := directory
  let some target ← realPath? proofFile | return false
  let mut index? := none
  for index in [0:commands.size] do
    let some named := proofFilePragma? commands[index]! | continue
    if (← realPath? (directory / named)) == some target then
      index? := some index
      break
  let some index := index? | return false
  -- A source changed since the rendering leaves the context stale.
  let mut changed := #[]
  for (source, hash) in sources do
    let current ← try some <$> contentHash (directory / source) catch _ => pure none
    unless current == some (UInt64.ofNat hash) do changed := changed.push source
  unless changed.isEmpty do
    logWarningAt ref m!"`{rendering}` is older than {", ".intercalate
      (changed.toList.map (s!"`{·}`"))}: verify the source again, then restart this file"
  for message in log.toList do logMessage message
  let deferral := getDeferral (← getEnv)
  let scopes := (← get).scopes
  modifyEnv (setDeferral · .all)
  try
    withReader (fun context => { context with
        fileName := rendering.toString, fileMap := inputCtx.fileMap, snap? := none }) do
      withEnableInfoTree false do
        for command in commands[:index + 1] do elabCommand command
  finally
    modifyEnv (setDeferral · deferral)
    modify fun state => { state with scopes }
  let some module ← deferredFor? directory proofFile
    | throwErrorAt ref m!"`{rendering}` does not register the module of this file"
  select module.name
  return true

@[command_elab verexProofsCommand]
def elabProofs : CommandElab := fun stx => do
  let keywords := mkNullNode #[stx[0], stx[1], stx[2]]
  let some name := selectedProofModule? (← getEnv)
    | throwErrorAt keywords "`verex proofs where` follows the proof file's header, which \
        selects its module"
  let some module := (deferredModules (← getEnv)).find? name
    | throwErrorAt keywords m!"the module `{name}` is not registered"
  if module.verified then
    throwErrorAt keywords m!"the proofs of `{name}` are verified above"
  for item in stx[stx.getNumArgs - 1].getArgs do
    unless item.isOfKind ``verexVerifyItem || item.isOfKind ``verexTheoremItem ||
        item.isOfKind ``verexOpenItem do
      throwErrorAt item "a proof file holds `verify` items, theorems, and `open` items; \
        this item belongs to the module"
  let since ← unreportedCount
  verifyDeferredModule module stx
  relocateForeignMessages keywords since

attribute [command_code_action verexProofsCommand] verifyHereCodeAction

end VerexSL.Proofs
