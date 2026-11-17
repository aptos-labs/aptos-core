-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Elab.GuardMsgs
import VerexSL

/-!
# Snapshots of a frontend's rendering

A source fixture of `verex-e2e-tests` is verified through a VerexSL rendering
of its validated unit, which the driver writes out. The files of this
directory are those renderings, each with the diagnostics the fixture's `.exp`
baseline states, so that the shapes a frontend produces are gated by
elaborating VerexSL alone.

A baseline names the source file and its positions; a snapshot is elaborated
on its own and names itself. `#guard_rendering` therefore compares the message
texts alone, as `#guard_msgs` does, with every reference to a file and a
position inside a text replaced (`withoutSourceReferences`), and elaborates
its command with the options the driver elaborates a rendering with: every
target verified, every message reported in the file that raised it.

A snapshot is taken only of a fixture whose rendering reaches its baseline:
there is no marker for one that does not, and the generator
(`scripts/prover-smoke-rendering.py`) refuses the fixtures whose diagnostics
the driver makes and a lone elaboration cannot.
-/

namespace VerexSL.Tests.Rendered

open Lean Elab Command Lean.Elab.Tactic.GuardMsgs

/-- A message as `#guard_msgs` writes one: its severity, then its text. -/
private def messageText (message : Message) : BaseIO String := do
  let mut text ← message.data.toString
  unless message.caption == "" do text := message.caption ++ ":\n" ++ text
  if !("\n".isPrefixOf text) then text := " " ++ text
  if message.isTrace then text := "trace:" ++ text
  else match message.severity with
    | .information => text := "info:" ++ text
    | .warning => text := "warning:" ++ text
    | .error => text := "error:" ++ text
  if text.isEmpty || text.back != '\n' then text := text ++ "\n"
  return text

/-- The options the source-verification driver elaborates a rendering with. -/
private def renderingOptions (options : Options) : Options :=
  (options.setBool `verex.skip_verify false).setBool `verex.relocateMessages false

/-- The messages of a command elaborated as a rendering's are, written as
`#guard_msgs` writes them, and the log they came in. -/
private def renderingMessages (cmd : Syntax) : CommandElabM (String × MessageLog) := do
  let messages ← withScope (fun scope => { scope with opts := renderingOptions scope.opts })
    (runAndCollectMessages cmd)
  let texts ← messages.toList.filter (!·.isSilent) |>.mapM (messageText ·)
  return ("---\n".intercalate texts |>.trimAscii |>.copy, messages)

/-- Whether a token's last two `:`-parts are numerals after a non-empty
name: a reference to a file and a position. -/
private def isSourceReference (token : List Char) : Bool :=
  let numeral (part : String) := !part.isEmpty && part.all Char.isDigit
  match (String.mk token).splitOn ":" |>.reverse with
  | column :: line :: name => numeral column && numeral line && name.any (!·.isEmpty)
  | _ => false

/-- A token with a reference to a file and a position replaced, the
punctuation that follows it kept. -/
private def withoutReference (token : String) : String :=
  let reversed := token.toList.reverse
  let trailing := reversed.takeWhile (",.;)]`'".toList.contains ·)
  if isSourceReference (reversed.drop trailing.length).reverse then
    "<source>" ++ String.mk trailing.reverse
  else token

/-- A message's text with every reference to a file and a position in it
replaced. A baseline's references name the source file or the rendering the
driver wrote beside it; a snapshot elaborated on its own names itself, so
the two agree on everything but these. -/
def withoutSourceReferences (text : String) : String :=
  let (out, token) := text.foldl (init := ("", "")) fun (out, token) character =>
    if character.isWhitespace then
      (out ++ withoutReference token ++ character.toString, "")
    else (out, token.push character)
  out ++ withoutReference token

-- A name with a colon in it is no reference; a file with a line and a column is.
#guard withoutSourceReferences "the clause `a:1` of `f:x`" == "the clause `a:1` of `f:x`"
#guard withoutSourceReferences "the operation `index` at a/b.rs:36:9 may panic" ==
  "the operation `index` at <source> may panic"
#guard withoutSourceReferences "at .verex/c.move.lean:7:74,\nand at d.rs:1:2" ==
  "at <source>,\nand at <source>"

/-- A baseline as its docstring states it. -/
private def baselineText (doc? : Option (TSyntax ``Lean.Parser.Command.docComment)) :
    CommandElabM String :=
  return (← doc?.mapM (getDocStringText ·)).getD "" |>.trimAscii |>.copy
    |> removeTrailingWhitespaceMarker

/-- Whether a rendering's messages are the baseline, both normalized as
`#guard_msgs` normalizes them. -/
private def reaches (baseline actual : String) : Bool :=
  let normalize (text : String) :=
    WhitespaceMode.normalized.apply (withoutSourceReferences text)
  normalize baseline == normalize actual

/-- The command's diagnostics are its fixture's baseline, which the docstring
holds: one message per `---` block, each the severity and the text, as
`#guard_msgs` writes them. -/
syntax (name := guardRenderingCommand)
  (docComment)? "#guard_rendering" "in" command : command

@[command_elab guardRenderingCommand]
def elabGuardRendering : CommandElab
  | `(command| $[$doc?:docComment]? #guard_rendering%$marker in $cmd) => do
      let baseline ← baselineText doc?
      let (actual, messages) ← renderingMessages cmd
      if reaches baseline actual then
        modify fun state => { state with messages := .empty }
      else
        modify fun state => { state with messages }
        logErrorAt marker m!"the rendering's diagnostics are not its fixture's baseline:\n\n\
          {actual}"
  | _ => throwUnsupportedSyntax

end VerexSL.Tests.Rendered
