-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

open VerexIR.Proofs.Denote

-- Transport of a concrete address is the identity, including when partially
-- applied under an array map in a generic vector search.
@[vir_denote_norm] theorem address_transport [Carriers] (θ : TypeArgs) :
    NTy.toSkolem θ .address = id := rfl

-- Generic helper calls also transport their parameter values by identity.
@[vir_denote_norm] theorem parameter_transport [Carriers] (θ : TypeArgs) (index : Nat) :
    NTy.toSkolem θ (.param index) = id := rfl

verify acquire by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (repeat' verex_split)
  all_goals simp_all
  all_goals grind

verify acquire_linear by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals (repeat' verex_split)
  all_goals simp_all
  all_goals grind

-- Instantiating a phantom parameter cannot merge two different resource
-- declarations. A write to one therefore preserves every slot of the other.
theorem set_distinct_struct {unit} [Θ : Skolems unit]
    (memory : Memory unit) {h k : StructHandle}
    (a b f g xs ys : NRow) (key otherKey : StorageKey)
    (value : Option ((⟨Θ.resolve (.struct h a f), xs, .value⟩ : ResourceType).carrier unit))
    (different : k ≠ h) :
    memory.set ⟨Θ.resolve (.struct h a f), xs, .value⟩ key value
      ⟨Θ.resolve (.struct k b g), ys, .value⟩ otherKey =
    memory ⟨Θ.resolve (.struct k b g), ys, .value⟩ otherKey := by
  apply Memory.set_other
  intro eq
  have same := congrArg (fun r : ResourceType => r.type.handle?) eq
  simp only [NTy.handle?_resolve_struct, Option.some.injEq] at same
  exact different same

verify delegate by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals try simp (disch := decide) [set_distinct_struct,
    Skolems.ofRuntime_toRuntime] at *
  all_goals grind

verify revoke by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals try simp (disch := decide) [set_distinct_struct,
    findIndex?_eq_some_iff,
    findIndex?_eq_none_iff] at *
  all_goals grind
