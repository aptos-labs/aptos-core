-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Retained-source analysis control

Shared, structured control flow for analyses over retained Move source.  The
event type is an analysis-specific projection: borrow checking, effects, gas,
or another abstract interpretation can share source points and topology
without depending on compiler lowering or on one another's abstract domain.

Loops remain source loops.  They are not rewritten to function recursion;
their backedge is explicit so each analysis can validate a post-fixpoint.
-/

namespace Move.Verify.Source

/--
An opaque analysis-site identifier, not a source coordinate. The retained-source
extractor maps each point to its original `Syntax`, which carries the complete
source range used for diagnostics.
-/
abbrev Point := Nat

inductive Control (Event : Type) where
  | done
  | abort
  | event (point : Point) (event : Event) (next : Control Event)
  | branch (point : Point)
      (thenBranch elseBranch next : Control Event)
  | loop (point : Point) (body next : Control Event)
  deriving Repr, BEq, DecidableEq, Inhabited, Lean.ToExpr

end Move.Verify.Source
