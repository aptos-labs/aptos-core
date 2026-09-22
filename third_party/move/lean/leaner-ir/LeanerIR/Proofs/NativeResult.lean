-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation

/-! Typed success/failure results lifted into native computations. -/

namespace LeanerIR.Proofs

def Spec.ofExcept (result : Except Error Result) : Spec State Error Result :=
  match result with
  | .ok value => Spec.pure value
  | .error error => Spec.abort error

theorem wp_ofExcept (result : Except Error Result)
    (ensures : Result → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (Spec.ofExcept result) ensures aborts initial ↔
      match result with
      | .ok value => ensures value initial
      | .error error => aborts error := by
  cases result <;> simp [Spec.ofExcept]

theorem StatePreserving.ofExcept (result : Except Error Result) :
    StatePreserving (Spec.ofExcept (State := State) result) := by
  cases result with
  | ok value => exact StatePreserving.pure value
  | error error => intro _ _ _ executed; exact executed.elim

end LeanerIR.Proofs
