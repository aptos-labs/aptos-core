-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "apply_carrier_determined.move"
verex proofs where

open VerexIR.Proofs VerexIR.Proofs.Denote

-- A postcondition may be established together with a fact of the final state.
theorem wp_with_final_fact {σ ε α : Type} (fact : σ → Prop) {action : Spec σ ε α}
    {ensures : α → σ → Prop} {aborts : ε → Prop} {initial : σ}
    (established : wp action (fun result final => ensures result final ∧ fact final) aborts
      initial) :
    wp action ensures aborts initial :=
  wp_mono established (fun _ _ both => both.1) (fun _ failed => failed)

verify determined_native_twice by lean =>
  verex_obligations
  verex_on
  | ensures_1 => verex_prepare; omega
  | _ => verex_decide

-- The vector result keeps the run `result_of` reads from being derived by termination.
-- Owing a trivial fact that reads `result_of` of the function value makes the direct
-- call of the same target record its run as that value's.
verify prelude_native_through_hof by lean =>
  obtain ⟨read, reads⟩ : ∃ read : ∀ {α : Type}, α → Prop, ∀ {α : Type} (value : α), read value :=
    ⟨fun _ => True, fun _ => trivial⟩
  apply wp_with_final_fact (fun final =>
    read (@ResultOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ ‹VerexIR.Validation.ExecutableUnit unit› (Skolems.runtime unit))
      (.cons (.vector (.int 8 false)) .nil) { parameters := [false], results := [false] }
      (.one (.vector (.int 8 false))) (FunctionValue.site1 ()) (b, ()) final ⟨#[], by decide⟩))
  verex_obligations
  verex_on
  | ensures_1 => verex_prepare; simp only [*]
  | _ => first | exact reads _ | verex_decide
