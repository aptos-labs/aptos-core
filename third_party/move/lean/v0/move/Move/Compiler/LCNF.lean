-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Lean.Compiler.LCNF.Main
import Lean.Compiler.LCNF.PrettyPrinter

/-!
# Access to Lean's normalized compiler IR

Leaner consumes base LCNF rather than raw elaborator expressions.  This gives
the backend a small, ANF-like language after notation, type classes, `do`, and
the `StateM` presentation of `Action` have been elaborated.
-/

namespace Move.Compiler

open Lean Elab Command
open Lean.Compiler.LCNF

/-- Compile `declName` and return the base LCNF declaration retained by Lean. -/
def getBaseDecl (declName : Name) : CoreM (Decl .pure) := do
  if let some decl ← getBaseDecl? declName then
    return decl
  Lean.compileDecls #[declName]
  let some decl ← getBaseDecl? declName
    | throwError "Lean did not retain base LCNF for `{declName}`"
  return decl

/-- Developer aid: show precisely the LCNF consumed by the Leaner backend. -/
syntax (name := printLeanerLCNF) "#print_leaner_lcnf " ident : command

elab_rules : command
  | `(#print_leaner_lcnf $decl:ident) => do
      let declName ← liftCoreM <| resolveGlobalConstNoOverload decl
      liftCoreM do
        let ir ← getBaseDecl declName
        logInfo (← ppDecl' ir .base)

end Move.Compiler
