-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Term

namespace LeanerIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- A finite semantic approximant with a separate proof-unrolling allowance.
The allowance never truncates executions: exhaustion must be proved unreachable.
Keeping this marker folded lets the closer expose one iteration at a time. -/
@[irreducible] def loopUnroll {ρ : ResultShape} {Γ : NRow} (_site _remaining fuel : Nat)
    (iteration : (HEnv Γ → Comp unit (Flow ρ Γ Unit)) → HEnv Γ → Comp unit (Flow ρ Γ Unit))
    (entry : HEnv Γ) : Comp unit (Flow ρ Γ Unit) :=
  Spec.fixApprox iteration fuel entry

/-- Every finite execution is covered, not just executions shorter than the
requested unrolling bound. No monotonicity or termination assumption is needed. -/
theorem wp_loopAt_unroll {ρ : ResultShape} {Γ : NRow} (site bound : Nat)
    (iteration : (HEnv Γ → Comp unit (Flow ρ Γ Unit)) → HEnv Γ → Comp unit (Flow ρ Γ Unit))
    (entry : HEnv Γ) (ensures : Flow ρ Γ Unit → Memory unit → Prop)
    (aborts : Failure → Prop) (initial : Memory unit)
    (allFuel : ∀ fuel, wp (loopUnroll site (bound + 1) fuel iteration entry) ensures aborts initial) :
    wp (loopAt site iteration entry) ensures aborts initial := by
  simp only [loopUnroll] at allFuel
  exact ⟨fun result final ⟨fuel, execution⟩ => (allFuel fuel).1 result final execution,
    fun error ⟨fuel, execution⟩ => (allFuel fuel).2.1 error execution,
    fun ⟨fuel, undefined⟩ => (allFuel fuel).2.2 undefined⟩

/-- Zero semantic fuel has no outcomes; a successor exposes one body step.
The next semantic fuel remains universally quantified. -/
theorem wp_loopUnroll_step {ρ : ResultShape} {Γ : NRow} (site remaining fuel : Nat)
    (iteration : (HEnv Γ → Comp unit (Flow ρ Γ Unit)) → HEnv Γ → Comp unit (Flow ρ Γ Unit))
    (entry : HEnv Γ) (ensures : Flow ρ Γ Unit → Memory unit → Prop)
    (aborts : Failure → Prop) (initial : Memory unit)
    (step : ∀ nextFuel, wp (iteration (loopUnroll site remaining nextFuel iteration) entry)
      ensures aborts initial) :
    wp (loopUnroll site (remaining + 1) fuel iteration entry) ensures aborts initial := by
  unfold loopUnroll at step ⊢
  cases fuel with
  | zero => exact ⟨fun _ _ h => h.elim, fun _ h => h.elim, fun h => h.elim⟩
  | succ fuel => exact step fuel

/-- The proof allowance is not a semantic cutoff. A path reaching its end
must be contradictory; neither divergence nor an arbitrary abort is assumed. -/
theorem wp_loopUnroll_exhausted {ρ : ResultShape} {Γ : NRow} (site fuel : Nat)
    (iteration : (HEnv Γ → Comp unit (Flow ρ Γ Unit)) → HEnv Γ → Comp unit (Flow ρ Γ Unit))
    (entry : HEnv Γ) (ensures : Flow ρ Γ Unit → Memory unit → Prop)
    (aborts : Failure → Prop) (initial : Memory unit) (unreachable : False) :
    wp (loopUnroll site 0 fuel iteration entry) ensures aborts initial :=
  unreachable.elim

end LeanerIR.Proofs.Denote
