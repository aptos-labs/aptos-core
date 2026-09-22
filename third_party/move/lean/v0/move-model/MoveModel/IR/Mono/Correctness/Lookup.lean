-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Transform

/-!
# Structural facts about monomorphization

These lemmas expose the definitional relationship between a generated
declaration and its source instance. They deliberately do not depend on the
runtime-tag coverage argument or on operational semantics.
-/

namespace MoveModel.IR

/-- A successful bounded module lookup is a lookup in the semantic program. -/
theorem Module.decl?_eq_some_program {m : Module} {f : FunId} {d : FunDecl}
    (h : m.decl? f = some d) : m.program.funs f = some d := by
  unfold Module.decl? at h
  split at h <;> simp_all

/-- Successful bounded lookup also proves that the id lies in the module. -/
theorem Module.lt_numFuns_of_decl?_eq_some {m : Module} {f : FunId} {d : FunDecl}
    (h : m.decl? f = some d) : f < m.numFuns := by
  unfold Module.decl? at h
  split at h <;> simp_all

/-- Characterize the declaration built for one plan entry. -/
theorem Module.specializeFun?_eq_some_iff {m : Module} {plan : MonoPlan}
    {key : MonoKey} {d : FunDecl} :
    m.specializeFun? plan key = some d ↔
      ∃ source, m.decl? key.funId = some source ∧
        d = { source.instantiate key.typeArgs with
          body := plan.rewriteCfg (source.instantiate key.typeArgs).body } := by
  constructor
  · intro h
    cases hsource : m.decl? key.funId with
    | none => simp [Module.specializeFun?, hsource] at h
    | some source =>
        have hd :
            { source.instantiate key.typeArgs with
              body := plan.rewriteCfg (source.instantiate key.typeArgs).body } = d := by
          simpa [Module.specializeFun?, hsource] using h
        exact ⟨source, rfl, hd.symm⟩
  · rintro ⟨source, hsource, rfl⟩
    simp [Module.specializeFun?, hsource]

/-- Looking up a generated function first selects its plan entry and then
specializes the corresponding source declaration. -/
@[simp] theorem Module.monomorphize_funs (m : Module) (plan : MonoPlan)
    (generated : FunId) :
    (m.monomorphize plan).program.funs generated = (do
      let key ← plan.entries[generated]?
      m.specializeFun? plan key) := rfl

/-- Monomorphization leaves structure declarations unchanged. -/
@[simp] theorem Module.monomorphize_structs (m : Module) (plan : MonoPlan) :
    (m.monomorphize plan).program.structs = m.program.structs := rfl

/-- Every generated declaration is associated with an entry at the same
position in the plan. -/
theorem Module.monomorphize_fun_eq_some_iff {m : Module} {plan : MonoPlan}
    {generated : FunId} {d : FunDecl} :
    (m.monomorphize plan).program.funs generated = some d ↔
      ∃ key, plan.entries[generated]? = some key ∧
        m.specializeFun? plan key = some d := by
  simp only [Module.monomorphize_funs]
  constructor
  · intro h
    cases hkey : plan.entries[generated]? with
    | none => simp [hkey] at h
    | some key =>
        refine ⟨key, rfl, ?_⟩
        simpa [hkey] using h
  · rintro ⟨key, hkey, hspecialized⟩
    rw [hkey]
    exact hspecialized

/-- Generated declarations are binder-free. -/
theorem Module.monomorphize_fun_typeParams_empty {m : Module} {plan : MonoPlan}
    {generated : FunId} {d : FunDecl}
    (h : (m.monomorphize plan).program.funs generated = some d) :
    d.typeParams = [] := by
  rw [Module.monomorphize_fun_eq_some_iff] at h
  obtain ⟨key, _, hspecialized⟩ := h
  exact specializeFun_typeParams_empty hspecialized

end MoveModel.IR
