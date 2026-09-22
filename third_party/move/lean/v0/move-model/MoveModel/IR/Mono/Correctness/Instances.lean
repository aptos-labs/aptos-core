-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Correctness.Semantics

/-!
# Generated-instance lookup

These lemmas connect a runtime specialization key to the declaration stored
at its generated function id. They isolate positional-plan lookup from the
later execution proof.
-/

namespace MoveModel.IR

/-- Resolving a key yields a generated declaration built from a
runtime-equivalent entry and the corresponding source declaration. -/
theorem Module.generatedDecl_of_generatedFunId?_eq_some
    {m : Module} {plan : MonoPlan} {key : MonoKey} {generated : FunId}
    {source : FunDecl}
    (hgenerated : plan.generatedFunId? key = some generated)
    (hsource : m.decl? key.funId = some source) :
    ∃ entry generatedDecl,
      plan.entries[generated]? = some entry ∧
      entry.RuntimeEq key ∧
      (m.monomorphize plan).program.funs generated = some generatedDecl ∧
      generatedDecl =
        { source.instantiate entry.typeArgs with
          body := plan.rewriteCfg (source.instantiate entry.typeArgs).body } := by
  obtain ⟨entry, hentry, heq⟩ :=
    plan.entry_of_generatedFunId?_eq_some hgenerated
  have hentrySource : m.decl? entry.funId = some source := by
    rw [heq.1]
    exact hsource
  let generatedDecl : FunDecl :=
    { source.instantiate entry.typeArgs with
      body := plan.rewriteCfg (source.instantiate entry.typeArgs).body }
  have hspecialized : m.specializeFun? plan entry = some generatedDecl := by
    simp [Module.specializeFun?, hentrySource, generatedDecl]
  have hlookup :
      (m.monomorphize plan).program.funs generated = some generatedDecl := by
    rw [Module.monomorphize_funs, hentry]
    exact hspecialized
  exact ⟨entry, generatedDecl, hentry, heq, hlookup, rfl⟩

/-- The generated body is the call-rewritten instance of the source body. -/
theorem Module.generatedDecl_body
    {m : Module} {plan : MonoPlan} {key : MonoKey} {generated : FunId}
    {source generatedDecl : FunDecl}
    (hgenerated : plan.generatedFunId? key = some generated)
    (hsource : m.decl? key.funId = some source)
    (hdecl : (m.monomorphize plan).program.funs generated =
      some generatedDecl) :
    ∃ entry,
      plan.entries[generated]? = some entry ∧
      entry.RuntimeEq key ∧
      generatedDecl.body =
        plan.rewriteCfg (source.body.instantiate entry.typeArgs) := by
  obtain ⟨entry, built, hentry, heq, hlookup, hbuilteq⟩ :=
    m.generatedDecl_of_generatedFunId?_eq_some hgenerated hsource
  rw [hdecl] at hlookup
  cases hlookup
  rw [hbuilteq]
  exact ⟨entry, hentry, heq, rfl⟩

/-- Materialization preserves the source declaration's runtime arity. -/
theorem Module.generatedDecl_numParams
    {m : Module} {plan : MonoPlan} {key : MonoKey} {generated : FunId}
    {source generatedDecl : FunDecl}
    (hgenerated : plan.generatedFunId? key = some generated)
    (hsource : m.decl? key.funId = some source)
    (hdecl : (m.monomorphize plan).program.funs generated =
      some generatedDecl) :
    generatedDecl.numParams = source.numParams := by
  obtain ⟨entry, built, _, _, hlookup, hbuilteq⟩ :=
    m.generatedDecl_of_generatedFunId?_eq_some hgenerated hsource
  rw [hdecl] at hlookup
  cases hlookup
  rw [hbuilteq]
  rfl

end MoveModel.IR
