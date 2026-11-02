-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term

/-!
# Connecting incoming edges to a shared continuation

The normal exits of an action establish a predicate over the value, locals
and memory at a join. The continuation is proved once under that predicate.
Abrupt flows retain their original destinations and postconditions.
-/

namespace VerexIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- The postcondition before a value continuation: normal exits establish
the join's entry predicate, while abrupt flows bypass the continuation. -/
def Flow.joinPost {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (entry : α → HEnv Γ → Memory unit → Prop)
    (ensures : Flow ρ Γ β → Memory unit → Prop) :
    Flow ρ Γ α → Memory unit → Prop
  | .value val env => entry val env
  | .return_ result env => ensures (.return_ result env)
  | .break_ nest env => ensures (.break_ nest env)
  | .continue_ nest env => ensures (.continue_ nest env)

/-- Compose an incoming region with a shared continuation proof. The entry
predicate may retain guarded alternatives from several incoming edges; it
must be established by the action, rather than assumed at the join. -/
theorem wp_flowBind_join {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (action : Comp unit (Flow ρ Γ α))
    (next : α → HEnv Γ → Comp unit (Flow ρ Γ β))
    (entry : α → HEnv Γ → Memory unit → Prop)
    (ensures : Flow ρ Γ β → Memory unit → Prop) (aborts : Failure → Prop)
    (state : Memory unit)
    (reaches : wp action (Flow.joinPost entry ensures) aborts state)
    (continues : ∀ value env state, entry value env state →
      wp (next value env) ensures aborts state) :
    wp (Flow.bind action next) ensures aborts state := by
  apply (wp_flowBind action next ensures aborts state).mpr
  apply wp_mono reaches
  · intro flow final reached
    cases flow with
    | value value env => exact continues value env final reached
    | return_ result env => exact reached
    | break_ nest env => exact reached
    | continue_ nest env => exact reached
  · intro error allowed
    exact allowed

end VerexIR.Proofs.Denote
