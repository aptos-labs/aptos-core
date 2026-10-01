-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerLang
import LeanerMove.Xir.Json
import LeanerMove.Xir.Lower

/-!
# Emitting XIR from a Lean source

A `.lean` source handed to compiler-v2 contributes the one Move module it
declares: at the end of the file, when `LEANER_XIR_OUTPUT` names a path, the
module registered in the file is lowered and written there. `#leaner_xir m`
prints a registered module's XIR instead.
-/

namespace LeanerIR.Move.Xir

open Lean Elab Command
open LeanerIR.Validation

/-- Syntax spanning an LIR location, so a message lands on its source. -/
private def locatedSyntax (unit : ValidatedUnit) (loc : Option LocId) (fallback : Syntax) :
    Syntax :=
  match loc.bind (unit.tables.locations[·.index]?) |>.bind (·.primary) with
  | some range => Syntax.atom (SourceInfo.synthetic ⟨range.startByte⟩ ⟨range.endByte⟩) ""
  | none => fallback

/-- Lower a unit's Move module, reporting a failure at its source. The
borrow analysis must have certified every function: its rejections are
Leaner's verdict on the program. -/
def compileUnit (ref : Syntax) (unit : ValidatedUnit) : CommandElabM (Option Module) := do
  let errors := unit.borrowDiagnostics.filter (·.severity == .error)
  for diagnostic in errors do
    logErrorAt (locatedSyntax unit diagnostic.primary ref)
      s!"{diagnostic.code}: {diagnostic.message}"
  unless errors.isEmpty do return none
  let modules := unit.namespaces.zipIdx.filter fun (ns, _) => ns.profile == some .move
  let #[(_, index)] := modules
    | logErrorAt ref "a unit compiles to Move bytecode when it declares exactly one Move module"
      return none
  match lowerModule unit ⟨index⟩ with
  | .ok module => return some module
  | .error failure =>
      logErrorAt (locatedSyntax unit failure.loc ref) failure.message
      return none

syntax (name := leanerXir) "#leaner_xir " ident : command

/-- Print the XIR of a registered module. -/
@[command_elab leanerXir]
def elabLeanerXir : CommandElab := fun stx => do
  let name := stx[1].getId
  let some unit := LeanerLang.registeredUnit? (← getEnv) name
    | throwErrorAt stx[1] s!"unknown Leaner module `{name}`"
  if let some module ← compileUnit stx[1] unit then
    logInfoAt stx[1] module.toJson.pretty

/-- Write the file's Move module where compiler-v2 asks for it. -/
@[command_elab Lean.Parser.Command.eoi]
def emitAtEndOfInput : CommandElab := fun stx => do
  let some path ← IO.getEnv "LEANER_XIR_OUTPUT" | return
  let units := LeanerLang.moduleUnits (← getEnv)
  let [(_, unit)] := units
    | unless units.isEmpty do
        logErrorAt stx "a `.lean` compiler input declares exactly one Leaner module"
      return
  let some module ← compileUnit stx unit | return
  if ← (System.FilePath.mk path).pathExists then
    throwErrorAt stx "the XIR output already exists"
  IO.FS.writeFile path module.toJson.compress

end LeanerIR.Move.Xir
