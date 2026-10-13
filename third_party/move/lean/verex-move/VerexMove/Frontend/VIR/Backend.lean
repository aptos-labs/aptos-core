-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Frontend.VIR.Encode
import VerexMove.Frontend.VIR.Decode

/-!
# Checked Move/Verex backend boundary

The public frontend/backend join is `ValidatedUnit`.  The current canonical
Verex printer is retained behind a projection from validated VIR while the
printer implementation is migrated away from its historical XAST-shaped
view.  That projection is lossless and has no access to the frontend input.
-/

namespace VerexMove.Frontend.VIR.Backend

open VerexMove.Frontend.Effects

/-- Render diagnostics with their primary byte ranges, so reports over a
whole unit keep distinct sites distinct. -/
def renderDiagnostics (tables : VerexIR.Tables)
    (diagnostics : Array VerexIR.Validation.Diagnostic) : String :=
  "\n".intercalate <| diagnostics.toList.map fun diagnostic =>
    let located := do
      let locId ← diagnostic.primary
      let location ← tables.locations[locId.index]?
      let range ← location.primary
      pure s!" at [{range.startByte}, {range.endByte})"
    s!"{diagnostic.code}: {diagnostic.message}{located.getD ""}"

/-- Import compiler-v2 XAST and run the shared Move-profile checker. -/
def fromXast (package : Package) : Except String VerexIR.Validation.ValidatedUnit := do
  let raw ← Encode.package package
  match VerexIR.Move.validate raw with
  | .ok checked => pure checked
  | .error diagnostics => throw (renderDiagnostics raw.tables diagnostics)

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

end VerexMove.Frontend.VIR.Backend
