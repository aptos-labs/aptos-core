-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Ids

namespace LeanerIR.Validation

inductive Severity where
  | error
  | warning
  | info
  deriving Repr, BEq, DecidableEq, Inhabited

structure RelatedLocation where
  loc : LocId
  message : String
  deriving Repr, BEq, Inhabited

structure Diagnostic where
  severity : Severity := .error
  code : String
  message : String
  primary : Option LocId := none
  related : Array RelatedLocation := #[]
  deriving Repr, BEq, Inhabited

def Diagnostic.error (code message : String) (primary : Option LocId := none) : Diagnostic :=
  { code, message, primary }

def Diagnostic.at (code message : String) (loc : LocId) : Diagnostic :=
  .error code message (some loc)

/-- Collapse exact duplicates — same severity, code, message, and locations —
preserving first-occurrence order. Distinct sites stay distinct through their
locations; only genuinely repeated reports collapse. -/
def dedupDiagnostics (diagnostics : Array Diagnostic) : Array Diagnostic :=
  diagnostics.foldl (init := #[]) fun kept diagnostic =>
    if kept.contains diagnostic then kept else kept.push diagnostic

end LeanerIR.Validation
