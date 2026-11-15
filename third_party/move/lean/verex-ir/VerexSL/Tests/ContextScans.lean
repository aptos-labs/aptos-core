-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A scan over the context ends where a hypothesis it handled stays because
something depends on it. -/

set_option maxHeartbeats 20000

/-- `verex_consume` specializes the implication once. -/
example (P Q : Prop) (R : (P → Q) → Prop) (premise : P) (implication : P → Q)
    (_dependent : R implication) : Q := by
  verex_consume
  assumption

/-- `verex_subst_equations` states what the equivalence decides once. -/
example (Q : Prop) (R : (true = true ↔ Q) → Prop) (equivalence : true = true ↔ Q)
    (_dependent : R equivalence) : Q := by
  verex_subst_equations
  assumption
