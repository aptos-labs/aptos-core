-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Correctness.Lookup
import MoveModel.IR.Mono.Correctness.Types

/-!
# Plan and call-closure correctness

This module turns the proof-facing call-closure certificate into concrete
generated function ids. Coverage of the runtime-tag quotient is independent
and is developed separately.
-/

namespace MoveModel.IR

/-- In a finite well-formed module, every semantic function declaration lies
inside the declared function range. -/
theorem Module.lt_numFuns_of_funs_eq_some {m : Module}
    (hfinite : m.FiniteWellFormed) {f : FunId} {d : FunDecl}
    (h : m.program.funs f = some d) : f < m.numFuns := by
  cases Nat.lt_or_ge f m.numFuns with
  | inl hlt => exact hlt
  | inr hge =>
      have hnone := (hfinite.2.2.2 f hge).1
      rw [h] at hnone
      cases hnone

/-- Semantic lookup and bounded lookup coincide for declarations of a finite
well-formed module. -/
theorem Module.decl?_eq_some_of_funs_eq_some {m : Module}
    (hfinite : m.FiniteWellFormed) {f : FunId} {d : FunDecl}
    (h : m.program.funs f = some d) : m.decl? f = some d := by
  simp [Module.decl?, m.lt_numFuns_of_funs_eq_some hfinite h, h]

/-- A certificate-resolved call has a generated target in the plan. -/
theorem MonoPlan.Certificate.generatedTargetForCall {m : Module}
    {plan : MonoPlan} (certificate : plan.Certificate m)
    {caller : MonoKey} (hcaller : caller ∈ plan.entries)
    {d : FunDecl} (hdecl : m.program.funs caller.funId = some d)
    {call : MonoKey} (hcall : call ∈ d.calledInstances) :
    ∃ generated,
      plan.generatedFunId?
        ⟨call.funId, instantiateTypes caller.typeArgs call.typeArgs⟩ =
          some generated := by
  obtain ⟨callee, hcallee, heq⟩ :=
    certificate.callClosure caller hcaller d hdecl call hcall
  exact plan.generatedFunId?_isSome_of_runtimeEq_mem hcallee heq

/-- The generated target selected for a certified call points back to a plan
entry with the expected source function and runtime type tags. -/
theorem MonoPlan.Certificate.generatedEntryForCall {m : Module}
    {plan : MonoPlan} (certificate : plan.Certificate m)
    {caller : MonoKey} (hcaller : caller ∈ plan.entries)
    {d : FunDecl} (hdecl : m.program.funs caller.funId = some d)
    {call : MonoKey} (hcall : call ∈ d.calledInstances) :
    ∃ generated entry,
      plan.generatedFunId?
        ⟨call.funId, instantiateTypes caller.typeArgs call.typeArgs⟩ =
          some generated ∧
      plan.entries[generated]? = some entry ∧
      entry.RuntimeEq
        ⟨call.funId, instantiateTypes caller.typeArgs call.typeArgs⟩ := by
  obtain ⟨generated, hgenerated⟩ :=
    certificate.generatedTargetForCall hcaller hdecl hcall
  obtain ⟨entry, hentry, heq⟩ :=
    plan.entry_of_generatedFunId?_eq_some hgenerated
  exact ⟨generated, entry, hgenerated, hentry, heq⟩

end MoveModel.IR
