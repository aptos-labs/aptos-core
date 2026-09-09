-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.LIR.Encode
import Transpiler.LIR.Decode

/-!
# Checked Move/Leaner backend boundary

The public frontend/backend join is `ValidatedUnit`.  The current canonical
Leaner printer is retained behind a projection from validated LIR while the
printer implementation is migrated away from its historical XAST-shaped
view.  That projection is lossless and has no access to the frontend input.
-/

namespace Transpiler.LIR.Backend

open Transpiler.Effects

def renderDiagnostics (diagnostics : Array LeanerIR.Validation.Diagnostic) : String :=
  "\n".intercalate <| diagnostics.toList.map fun diagnostic =>
    s!"{diagnostic.code}: {diagnostic.message}"

/-- Import compiler-v2 XAST and run the shared Move-profile checker. -/
def fromXast (package : Package) : Except String LeanerIR.Validation.ValidatedUnit := do
  let raw ← Encode.package package
  match LeanerIR.Move.validate raw with
  | .ok checked => pure checked
  | .error diagnostics => throw (renderDiagnostics diagnostics)

/-- The established printer's typed Move view, derived from checked LIR only.
The requested language profile is explicit: this transitional backend accepts
Move units and rejects every other selected profile rather than interpreting
the same neutral nodes with accidental Move semantics. -/
def toPrinterPackage (profile : LeanerIR.Profile)
    (unit : LeanerIR.Validation.ValidatedUnit) : Except String Package :=
  match profile with
  | .move =>
      if unit.namespaces.all (·.profile == some .move) then
        Decode.package unit
      else
        .error "the Move Leaner backend cannot print a namespace with another semantic profile"
  | .rust => .error "the Rust-profile Leaner backend is not implemented"
  | .extension _ => .error "no Leaner backend is registered for the selected extension profile"

end Transpiler.LIR.Backend
