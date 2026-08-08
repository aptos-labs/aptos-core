-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Move.Frontend.Repr
import Move.Frontend.Decode
import Move.Frontend.Elim

/-!
# The `masm%` and `move%` Elaborators

`masm% "<masm source>"` and `move% "<Move source>"` elaborate an embedded
program into a `Program`: at elaboration time they run the Aptos CLI's
`exchange` command (which parses/compiles the source with the real
assembler resp. compiler v2 — the latter with specifications from the
genuine `spec` blocks — and dumps the stackless-bytecode CFG as JSON),
decode the JSON, and splice the result as
the term `MProgram.toProgram <first-order literal>`.  Compiler and frontend
errors are reported as elaboration errors at the string literal.

The elaborators invoke the Aptos CLI's `aptos move exchange` command.
The CLI binary is located through the environment variable `APTOS_CLI`;
if it is unset, `aptos` is looked up on `PATH`.

This is the only module of the library that imports `Lean`; the theory
itself stays independent of the metaprogramming framework.
-/

namespace Move.Frontend

open Move.IR
open Lean

/-! ## Quotation: `ToExpr` instances for the first-order representation -/

deriving instance ToExpr for Ty
deriving instance ToExpr for RefRoot
deriving instance ToExpr for RefTarget
deriving instance ToExpr for Value
deriving instance ToExpr for QuantKind
deriving instance ToExpr for SpecBinop
deriving instance ToExpr for SpecExp
deriving instance ToExpr for Oper
deriving instance ToExpr for Instr
deriving instance ToExpr for Move.IR.Term
deriving instance ToExpr for Block
deriving instance ToExpr for MLoop
deriving instance ToExpr for MContract
deriving instance ToExpr for MFun
deriving instance ToExpr for MStruct
deriving instance ToExpr for MProgram

/-! ## Running the frontend -/

/-- Locates the Aptos CLI binary: `APTOS_CLI` if set, else `aptos` on
`PATH`. -/
def findFrontend : IO String := do
  if let some p ← IO.getEnv "APTOS_CLI" then
    return p
  return "aptos"

/-- The modification time of the CLI binary, resolving a bare command name
against `PATH`; `none` if it cannot be determined.  Part of the dump cache
key, so that rebuilding the CLI invalidates cached dumps. -/
def frontendStamp (exe : String) : IO (Option String) := do
  let stat (p : System.FilePath) : IO (Option String) := do
    try
      let md ← p.metadata
      return some s!"{md.modified.sec}.{md.modified.nsec}"
    catch _ =>
      return none
  if exe.contains '/' then
    stat exe
  else
    for dir in System.SearchPath.parse ((← IO.getEnv "PATH").getD "") do
      if let some stamp ← stat (dir / exe) then
        return some stamp
    return none

/-- Runs `aptos move exchange` on the given source (`fileArg` is
`--masm-file` or `--move-file`), returning its JSON dump.

Dumps are cached in the temp directory, keyed by the source content and
the CLI binary's modification time: re-elaborating an unchanged embedded
source does not re-run the CLI, and rebuilding the CLI invalidates the
cache.  `fresh` bypasses and refreshes the cache — used when a cached dump
fails to decode (e.g. torn by a concurrent build). -/
def runFrontend (fileArg : String) (suffix input : String)
    (fresh : Bool := false) : IO String := do
  let exe ← findFrontend
  let tmpDir := (← IO.getEnv "TMPDIR").getD "/tmp"
  let stamp ← frontendStamp exe
  let base := System.FilePath.mk tmpDir /
    s!"move-exchange-{hash (fileArg, input, stamp)}"
  let file := base.addExtension suffix
  let outFile := base.addExtension "json"
  -- Without a CLI stamp the key cannot see CLI changes; skip the cache.
  if !fresh && stamp.isSome && (← outFile.pathExists) then
    return ← IO.FS.readFile outFile
  IO.FS.writeFile file input
  let out ←
    try
      IO.Process.output
        { cmd := exe,
          args := #["move", "exchange", fileArg, file.toString,
            "--out-file", outFile.toString] }
    catch e =>
      throw (IO.userError s!"cannot run `{exe}`: {e}")
  if out.exitCode != 0 then
    throw (IO.userError s!"`{exe} move exchange {fileArg}` failed:\n\
      {out.stderr}{out.stdout}")
  IO.FS.readFile outFile

open Elab Term in
/-- Common elaboration of an embedded program string into the decoded
first-order `MProgram` value. -/
def decodeFrontend (s : Syntax) (fileArg suffix : String) :
    TermElabM MProgram := do
  let some input := s.isStrLit?
    | throwErrorAt s "expected a string literal"
  let run (fresh : Bool) : TermElabM String := do
    try
      runFrontend fileArg suffix input (fresh := fresh)
    catch e =>
      throwErrorAt s "`aptos move exchange` failed (set APTOS_CLI to \
        an `aptos` CLI binary, or put one on PATH):\n{e.toMessageData}"
  match decodeMProgram (← run false) with
  | .ok p => return p
  | .error _ =>
    -- A cached dump can be stale (a CLI change the stamp missed) or torn
    -- by a concurrent build; refresh the cache once before giving up.
    match decodeMProgram (← run true) with
    | .ok p => return p
    | .error e =>
      throwErrorAt s "cannot decode `aptos move exchange` output: {e}"

open Elab Term in
/-- Common elaboration of an embedded program string into the first-order
`MProgram` literal. -/
def elabFrontendM (s : Syntax) (fileArg suffix : String) :
    TermElabM Expr := do
  return toExpr (← decodeFrontend s fileArg suffix)

open Elab Term in
/-- Common elaboration of an embedded program string into a `Program`. -/
def elabFrontend (s : Syntax) (fileArg suffix : String) :
    TermElabM Expr := do
  return mkApp (mkConst ``MProgram.toProgram) (← elabFrontendM s fileArg suffix)

open Elab Term in
/-- Common elaboration of an embedded program string into a `Program`,
running the reference elimination (`MProgram.elim`) at elaboration time —
rejections of the pass surface as positioned errors. -/
def elabFrontendElim (s : Syntax) (fileArg suffix : String) :
    TermElabM Expr := do
  match (← decodeFrontend s fileArg suffix).elim with
  | .ok p' => return mkApp (mkConst ``MProgram.toProgram) (toExpr p')
  | .error e => throwErrorAt s "reference elimination failed: {e}"

open Elab Term in
/-- Elaborates an embedded masm string into a `Program` (see module docs). -/
elab "masm% " s:str : term => elabFrontend s "--masm-file" "masm"

open Elab Term in
/-- Elaborates an embedded masm string into the first-order `MProgram`,
which retains the declaration names (`masm%` is the composition with
`MProgram.toProgram`). -/
elab "masmM% " s:str : term => elabFrontendM s "--masm-file" "masm"

open Elab Term in
/-- Elaborates an embedded self-contained Move module into a `Program`
(see module docs). -/
elab "move% " s:str : term => elabFrontend s "--move-file" "move"

open Elab Term in
/-- Elaborates an embedded self-contained Move module into the first-order
`MProgram`, which retains the declaration names (`move%` is the composition
with `MProgram.toProgram`). -/
elab "moveM% " s:str : term => elabFrontendM s "--move-file" "move"

open Elab Term in
/-- Elaborates an embedded masm string into a `Program`, applying the
reference elimination — borrow-based masm verifies directly. -/
elab "masmElim% " s:str : term => elabFrontendElim s "--masm-file" "masm"

open Elab Term in
/-- Elaborates an embedded self-contained Move module into a `Program`,
applying the reference elimination — borrow-based Move source verifies
directly. -/
elab "moveElim% " s:str : term => elabFrontendElim s "--move-file" "move"

end Move.Frontend
