-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import MoveModel.Frontend.XIR
import MoveModel.Frontend.Decode
import MoveModel.Frontend.Elim

/-!
# The `masm%` and `move%` Elaborators

`masm% "<masm source>"` and `move% "<Move source>"` elaborate embedded source
into a `Program`.  At elaboration time they:

1. run `aptos move exchange`;
2. decode the resulting stackless-CFG JSON into an `MProgram`; and
3. splice `MProgram.toProgram <first-order literal>` into the Lean term.

Masm input goes through the real assembler.  Move input goes through compiler
v2, including genuine `spec` blocks.  Compiler and frontend failures are
reported at the source string.

`APTOS_MOVE_EXCHANGE` may name the lightweight, direct exchange frontend.
For backward compatibility, `APTOS_CLI` may name the full Aptos CLI. Otherwise
the elaborators use `aptos` on `PATH`.

This is the only module of the library that imports `Lean`; the theory
itself stays independent of the metaprogramming framework.
-/

namespace MoveModel.Frontend

open MoveModel.IR
open MoveModel.Frontend.XIR
open Lean

/-! ## Quotation: `ToExpr` instances for the first-order representation -/

deriving instance ToExpr for IntWidth
deriving instance ToExpr for NumType
deriving instance ToExpr for Ty
deriving instance ToExpr for TypeTagToken
deriving instance ToExpr for ResourceKey
deriving instance ToExpr for RefRoot
deriving instance ToExpr for RefTarget
deriving instance ToExpr for Value
deriving instance ToExpr for QuantKind
deriving instance ToExpr for SpecBinop
deriving instance ToExpr for SpecExp
deriving instance ToExpr for Oper
deriving instance ToExpr for Instr
deriving instance ToExpr for MoveModel.IR.Term
deriving instance ToExpr for Block
deriving instance ToExpr for AbilitySet
deriving instance ToExpr for TypeParamDecl
deriving instance ToExpr for MoveModel.IR.Visibility
deriving instance ToExpr for Dialect
deriving instance ToExpr for AttributeArg
deriving instance ToExpr for MoveModel.IR.Attribute
deriving instance ToExpr for SourceSpan
deriving instance ToExpr for BlockSourceMap
deriving instance ToExpr for FunSourceMap
deriving instance ToExpr for StructMeta
deriving instance ToExpr for FunMeta
deriving instance ToExpr for ExternalFunRef
deriving instance ToExpr for ExternalModuleRef
deriving instance ToExpr for MLoop
deriving instance ToExpr for MContract
deriving instance ToExpr for MFun
deriving instance ToExpr for MStruct
deriving instance ToExpr for MProgram
deriving instance ToExpr for MModule

/-! ## Running the frontend -/

/-- Locates the exchange frontend. `APTOS_MOVE_EXCHANGE` takes precedence and
names a binary whose arguments are the exchange flags directly. `APTOS_CLI`
selects the backward-compatible `aptos move exchange` frontend. -/
def findFrontend : IO (String × Array String) := do
  if let some p ← IO.getEnv "APTOS_MOVE_EXCHANGE" then
    return (p, #[])
  if let some p ← IO.getEnv "APTOS_CLI" then
    return (p, #["move", "exchange"])
  return ("aptos", #["move", "exchange"])

/-- Runs the exchange frontend on the given source (`fileArg` is `--masm-file`
or `--move-file`), returning its JSON dump.

Each elaboration uses fresh input and output paths and runs the CLI.  In
particular, no previously written `/tmp` dump is trusted as proof input. -/
def runFrontend (fileArg : String) (suffix input : String) : IO String := do
  let (exe, commandArgs) ← findFrontend
  IO.FS.withTempDir fun dir => do
    let file := dir / s!"input.{suffix}"
    let outFile := dir / "output.json"
    IO.FS.writeFile file input
    let out ←
      IO.Process.output
        { cmd := exe,
          args := commandArgs ++ #[fileArg, file.toString,
            "--out-file", outFile.toString] }
    if out.exitCode != 0 then
      throw (IO.userError s!"exchange frontend `{exe}` failed:\n\
        {out.stderr}{out.stdout}")
    IO.FS.readFile outFile

open Elab Term in
/-- Common elaboration of an embedded program string into the decoded
first-order `MProgram` value. -/
def decodeFrontend (s : Syntax) (fileArg suffix : String) :
    TermElabM MProgram := do
  let some input := s.isStrLit?
    | throwErrorAt s "expected a string literal"
  let run : TermElabM String := do
    try
      runFrontend fileArg suffix input
    catch e =>
      throwErrorAt s "Move exchange frontend failed (set APTOS_MOVE_EXCHANGE \
        for the lightweight frontend, or set APTOS_CLI):\n{e.toMessageData}"
  match decodeMProgram (← run) with
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
/-- Elaborates embedded masm into the result of executable interprocedural
reference elimination.  `refElim_correct` does not yet cover this cross-call
pipeline; the resulting `Program` is what subsequent theorems verify. -/
elab "masmElim% " s:str : term => elabFrontendElim s "--masm-file" "masm"

open Elab Term in
/-- Elaborates embedded self-contained Move into the result of executable
interprocedural reference elimination.  `refElim_correct` does not yet cover
this cross-call pipeline; the resulting `Program` is what subsequent theorems
verify. -/
elab "moveElim% " s:str : term => elabFrontendElim s "--move-file" "move"

end MoveModel.Frontend
