-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Move proof files

The proofs a Move file's module needs beyond its automatic verification live
in the proof file beside it, `foo.proof.lean` for `foo.move`:

```lean
import VerexMove.Proofs
verex proofs for "foo.move"
verex proofs where

verify f by
  …
```

`verex proofs for` names the Move file, relative to the proof file. Where the
source-verification driver elaborates the proof file, the module is
registered already. Elsewhere, as in the editor, the header loads it from the
rendering the last verification of the Move file wrote: the package's
`build/verex-verify.lean` for a package, `build/verex/<path>.lean` of the
package around a file verified on its own, and `.verex/foo.move.lean` beside a
file of no package.
-/

namespace VerexMove.Proofs

open Lean Elab Command

/-- The proof file of a Move source file. -/
def proofFile (source : System.FilePath) : System.FilePath :=
  source.withExtension "proof.lean"

/-- Where verifying a package writes its rendering. -/
def packageRendering (package : System.FilePath) : System.FilePath :=
  package / "build" / "verex-verify.lean"

private def realPath? (path : System.FilePath) : IO (Option System.FilePath) :=
  try some <$> IO.FS.realPath path catch _ => pure none

/-- The package around a source file: the nearest directory above it with a
`Move.toml`. -/
partial def packageOf? (source : System.FilePath) : IO (Option System.FilePath) := do
  let some directory ← realPath? (source.parent.getD ".") | return none
  let rec up (directory : System.FilePath) : IO (Option System.FilePath) := do
    if ← (directory / "Move.toml").pathExists then return some directory
    match directory.parent with
    | some parent => if parent == directory then return none else up parent
    | none => return none
  up directory

/-- Where verifying a Move file on its own writes its rendering: the build
directory of the package around it, out of the package's sources, whose Lean
files the package elaborates; beside a file of no package, the hidden
`.verex` directory. -/
def fileRendering (source : System.FilePath) : IO System.FilePath := do
  let name := source.fileName.getD "source"
  let some package ← packageOf? source
    | return (source.parent.getD ".") / ".verex" / s!"{name}.lean"
  let some resolved ← realPath? source | return package / "build" / "verex" / s!"{name}.lean"
  let relative := (resolved.toString.drop (package.toString.length + 1)).toString
  return package / "build" / "verex" / s!"{relative}.lean"

/-- `verex proofs for "foo.move"`, the header of the proof file of
`foo.move`: it establishes the context of the file's module. -/
syntax (name := verexMoveProofsHeader) "verex" &"proofs" "for" str : command

@[command_elab verexMoveProofsHeader]
def elabProofsHeader : CommandElab := fun stx => do
  let some named := stx[3].isStrLit? | throwErrorAt stx "expected the Move file"
  let here : System.FilePath := ← getFileName
  let source := (here.parent.getD ".") / named
  let some fileName := here.fileName | throwErrorAt stx "a proof file has a name"
  unless fileName.endsWith ".proof.lean" do
    throwErrorAt stx m!"a proof file is named after its Move file: `foo.proof.lean` for `foo.move`"
  unless (← realPath? (proofFile source)) == (← realPath? here) do
    throwErrorAt stx[3] m!"this file holds the proofs of \
      `{(fileName.dropEnd ".proof.lean".length).toString}.move`, not of `{named}`"
  if ← VerexSL.Proofs.selectRegistered here then return
  -- The latest rendering that renders the module.
  let mut candidates := #[← fileRendering source]
  if let some package ← packageOf? source then candidates := candidates.push (packageRendering package)
  let mut dated := #[]
  for candidate in candidates do
    if ← candidate.pathExists then dated := dated.push (candidate, (← candidate.metadata).modified)
  let latest := dated.qsort fun a b => compare a.2 b.2 == .gt
  let since ← VerexSL.Verify.unreportedCount
  for (rendering, _) in latest do
    if ← VerexSL.Proofs.loadRendering stx rendering here then
      VerexSL.Verify.relocateForeignMessages stx since
      return
  throwErrorAt stx m!"no rendering renders the module of `{named}` (looked for \
    {", ".intercalate (candidates.toList.map (s!"`{·}`"))}): verify it, with `verex-move verify` \
    or, in a package, `move prove --lean`, then restart this file"

end VerexMove.Proofs
