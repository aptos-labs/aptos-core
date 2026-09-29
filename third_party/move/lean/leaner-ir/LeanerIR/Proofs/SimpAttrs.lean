-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Verification attributes
-/

/-- Marks a generated typed twin structure.  The storage tactic destructures
a twin-typed witness down to its scalar fields so the erasure reduces to a
literal runtime value; the tag is what licenses that destructuring. -/
initialize LeanerIR.Proofs.leanerTwinAttribute : Lean.TagAttribute ←
  Lean.registerTagAttribute `leaner_twin
    "generated typed twin of an LIR struct declaration"
