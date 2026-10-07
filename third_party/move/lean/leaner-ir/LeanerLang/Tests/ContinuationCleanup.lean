-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

open LeanerIR.Proofs

-- An unused alias chain must not keep the whole saved computation alive.
-- Clearing only direct occurrences of `wp` leaves all three definitions.
example (action : Spec Unit Unit Unit) (value : Nat) : value = value := by
  let continuation := fun state => wp action (fun _ _ => True) (fun _ => True) state
  let alias := continuation
  let again := alias
  let ordinary := value + 1
  leaner_denote_clear_computations
  fail_if_success clear again
  fail_if_success clear alias
  fail_if_success clear continuation
  guard_hyp ordinary := value + 1
  rfl

-- A continuation still used by the goal or a hypothesis must survive.
example (action : Spec Unit Unit Unit) :
    wp action (fun _ _ => True) (fun _ => True) () →
      wp action (fun _ _ => True) (fun _ => True) () := by
  let continuation := fun state => wp action (fun _ _ => True) (fun _ => True) state
  let alias := continuation
  let again := alias
  change again () → again ()
  intro holds
  leaner_denote_clear_computations
  fail_if_success clear again
  exact holds
