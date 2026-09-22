-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeLoop

namespace LeanerIR.Proofs.NativeFlow

/-- Typed local state at an ordinary statement join or a loop-control exit.
The distinction is kept until the enclosing loop consumes it. -/
inductive Flow (Locals : Type) (LoopLocals : Type := Locals) where
  | normal (locals : Locals)
  | continue_ (locals : LoopLocals)
  | break_ (locals : LoopLocals)

def Flow.locals : Flow Locals → Locals
  | .normal locals | .continue_ locals | .break_ locals => locals

def sequence (first : Spec State Error (Flow Join LoopLocals))
    (next : Join → Spec State Error (Flow Locals LoopLocals)) : Spec State Error (Flow Locals LoopLocals) :=
  Spec.bind first fun
    | .normal locals => next locals
    | .continue_ locals => Spec.pure (.continue_ locals)
    | .break_ locals => Spec.pure (.break_ locals)

def Flow.step : Flow Locals → NativeLoop.Step Locals Locals
  | .normal locals | .continue_ locals => .next locals
  | .break_ locals => .done locals

def iteration (body : Spec State Error (Flow Locals)) :
    Spec State Error (NativeLoop.Step Locals Locals) where
  ok := fun initial result final => ∃ flow, body.ok initial flow final ∧ result = flow.step
  aborts := body.aborts
  undefined := body.undefined

theorem iteration_preserves (body : Spec State Error (Flow Locals))
    (preserves : StatePreserving body) : StatePreserving (iteration body) := by
  rintro initial result final ⟨flow, ran, _⟩
  exact preserves _ _ _ ran

theorem wp_iteration (body : Spec State Error (Flow Locals))
    (post : NativeLoop.Step Locals Locals → State → Prop) (aborts : Error → Prop)
    (initial : State) :
    wp (iteration body) post aborts initial ↔
      wp body (fun flow final => post flow.step final) aborts initial := by
  constructor
  · rintro ⟨normal, failed, defined⟩
    exact ⟨fun flow final ran => normal _ _ ⟨flow, ran, rfl⟩, failed, defined⟩
  · rintro ⟨normal, failed, defined⟩
    exact ⟨by rintro _ _ ⟨flow, ran, rfl⟩; exact normal _ _ ran, failed, defined⟩

end LeanerIR.Proofs.NativeFlow
