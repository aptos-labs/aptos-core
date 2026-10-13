-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.VIR.Encode
import Transpiler.VIR.Decode

/-!
# Checked Move/Verex backend boundary

The public frontend/backend join is `ValidatedUnit`.  The current canonical
Verex printer is retained behind a projection from validated VIR while the
printer implementation is migrated away from its historical XAST-shaped
view.  That projection is lossless and has no access to the frontend input.
-/

namespace Transpiler.VIR.Backend

open Transpiler.Effects

def renderDiagnostics (diagnostics : Array VerexIR.Validation.Diagnostic) : String :=
  "\n".intercalate <| diagnostics.toList.map fun diagnostic =>
    s!"{diagnostic.code}: {diagnostic.message}"

/-- Import compiler-v2 XAST and run the shared Move-profile checker. -/
def fromXast (package : Package) : Except String VerexIR.Validation.ValidatedUnit := do
  let raw ← Encode.package package
  match VerexIR.Move.validate raw with
  | .ok checked => pure checked
  | .error diagnostics => throw (renderDiagnostics diagnostics)

/-- The established printer's typed Move view, derived from checked VIR only.
The requested language profile is explicit: this transitional backend accepts
Move units and rejects every other selected profile rather than interpreting
the same neutral nodes with accidental Move semantics. -/
def toPrinterPackage (profile : VerexIR.Profile)
    (unit : VerexIR.Validation.ValidatedUnit) : Except String Package :=
  match profile with
  | .move =>
      if unit.namespaces.all (·.profile == some .move) then
        Decode.package unit
      else
        .error "the Move Verex backend cannot print a namespace with another semantic profile"
  | .rust => .error "the Rust-profile Verex backend is not implemented"
  | .extension _ => .error "no Verex backend is registered for the selected extension profile"

end Transpiler.VIR.Backend
