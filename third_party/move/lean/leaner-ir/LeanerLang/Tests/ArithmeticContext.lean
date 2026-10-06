-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

-- The fast path proves arithmetic without processing unrelated hypotheses.
example (x y : Int) (h : x ≤ y) (p : Prop) (_irrelevant : p ∨ ¬p) : x < y + 1 := by
  leaner_denote_arithmetic_only

-- Mixed propositions are deliberately outside the fast path. The full
-- solver must still be available when such a proposition supplies the bound.
example (x y : Int) (p : Prop) (h : x ≤ y ∧ p) : x ≤ y := by
  fail_if_success leaner_denote_arithmetic_only
  leaner_denote_omega
