-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeFlow

/-! Each nested loop adds its typed header to the target stack. Escaping
control carries the selected outer header, not an encoded value or frame. -/

namespace LeanerIR.Proofs.NativeNestedFlow

open NativeFlow

def step : Flow Locals (Sum Locals Outer) → NativeLoop.Step Locals (Flow Locals Outer)
  | .normal locals | .continue_ (.inl locals) => .next locals
  | .break_ (.inl locals) => .done (.normal locals)
  | .continue_ (.inr outer) => .done (.continue_ outer)
  | .break_ (.inr outer) => .done (.break_ outer)

def iteration (body : Spec State Error (Flow Locals (Sum Locals Outer))) :
    Spec State Error (NativeLoop.Step Locals (Flow Locals Outer)) where
  ok := fun initial result final => ∃ flow, body.ok initial flow final ∧ result = step flow
  aborts := body.aborts
  undefined := body.undefined

theorem iteration_preserves (body : Spec State Error (Flow Locals (Sum Locals Outer)))
    (preserves : StatePreserving body) : StatePreserving (iteration body) := by
  rintro initial result final ⟨flow, ran, _⟩
  exact preserves _ _ _ ran

theorem wp_iteration (body : Spec State Error (Flow Locals (Sum Locals Outer)))
    (post : NativeLoop.Step Locals (Flow Locals Outer) → State → Prop) (aborts : Error → Prop)
    (initial : State) :
    wp (iteration body) post aborts initial ↔
      wp body (fun flow final => post (step flow) final) aborts initial := by
  constructor
  · rintro ⟨normal, failed, defined⟩
    exact ⟨fun flow final ran => normal _ _ ⟨flow, ran, rfl⟩, failed, defined⟩
  · rintro ⟨normal, failed, defined⟩
    exact ⟨by rintro _ _ ⟨flow, ran, rfl⟩; exact normal _ _ ran, failed, defined⟩

end LeanerIR.Proofs.NativeNestedFlow
