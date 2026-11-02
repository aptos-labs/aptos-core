-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR.Proofs

-- Clauses at different source locations can use the same stated facts;
-- constructing the proof leaves their diagnostic wrappers intact.
example (p q : Prop) (known : Obligation "callee" 0 1 (p ∧ q)) :
    Obligation "caller" 0 1 q ∧ Obligation "caller" 2 3 p := by
  verex_stated

example (p q : Prop) (known : Obligation "call" 0 1 (¬(p ∨ q))) : ¬p ∧ ¬q := by
  verex_stated

-- The successful Boolean result of a call rules out the negative case of
-- its specified condition, before preparing unrelated invariant clauses.
example (condition : Bool) (q : Prop)
    (contract : Obligation "call" 0 1 (((true : Bool) = true) ↔ condition = true))
    (branch : ¬condition = true) : Obligation "invariant" 0 1 q := by
  verex_stated

example (p q : Prop) (contract : Obligation "call" 0 1 (p ↔ True)) (branch : ¬p) : q := by
  verex_stated

-- An unknown result cannot establish the condition. A failed attempt must
-- neither invent a contradiction nor consume the surrounding proof state.
example (condition : Bool) (p : Prop)
    (contract : Obligation "call" 0 1 (condition = true ↔ p)) (denied : ¬p) :
    ¬condition = true := by
  fail_if_success have : False := by verex_stated
  intro result
  exact denied ((Obligation_iff.mp contract).mp result)

example (p _q : Prop) (known : p) : p := by
  fail_if_success have : p ∧ _q := by verex_stated
  verex_stated
