-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Simp inventory of the denotation

One named set carries the unfolding of a compiled function's denotation
and the weakest-precondition rule of every native operation.  A `verify`
normalizes its goal with this set once, then closes leaves with decision
procedures; nothing in it selects a route or matches a goal shape.
-/

/-- Denotation unfolding and native weakest-precondition rules. -/
register_simp_attr lir_denote

/-- The general normal-form lemmas of verification conditions: logic,
encodings, runtime accessors, and injectivity.  Precomputed here so that a
normalization names only the per-target constants. -/
register_simp_attr lir_denote_norm
