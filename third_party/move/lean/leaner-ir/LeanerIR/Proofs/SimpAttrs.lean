-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Verification simp inventories

Attribute names carry a `lir_` prefix because the frozen reference stack
registers the unprefixed names and both stacks meet in one import closure.
-/

/-- Weakest-precondition normalization rules. -/
register_simp_attr lir_wp_norm

/-- Specification-level unfolds for generated contract and spec bodies. -/
register_simp_attr lir_spec_norm

/-- Runtime-data unfolds symbolic execution needs once a program point is
concrete: frame reads, literal reification, and the pure operation
evaluators. -/
register_simp_attr lir_data_norm

/-- Closed rows the native drive reconciles a program point with: frame and
state finalization, loan registries, write-backs, and the canonical array
spellings.  This is an attribute rather than a list written into the drive's
tactics because the drive applies it at every program point, and an inline
list is re-elaborated into a discrimination tree on each of those
applications — measured at about eight milliseconds a call for this
inventory, which was half the drive's time on a scalar function. -/
register_simp_attr lir_reconcile

/-- Evaluator computations at the row boundary: the closed evaluations a
row-route resolution step uses to turn an evaluator equation into
constructor form.  Registered once, for the registered-set reason recorded
on `lir_reconcile`. -/
register_simp_attr lir_row_eval

/-- Marks a generated typed twin structure.  The storage tactic destructures
a twin-typed witness down to its scalar fields so the erasure reduces to a
literal runtime value; the tag is what licenses that destructuring. -/
initialize LeanerIR.Proofs.leanerTwinAttribute : Lean.TagAttribute ←
  Lean.registerTagAttribute `leaner_twin
    "generated typed twin of an LIR struct declaration"

/-- Loan-freedom of a value, decided by simp: the constructor forms of
`Plain`, and every generated twin's `plain_erase`. -/
register_simp_attr leaner_plain
