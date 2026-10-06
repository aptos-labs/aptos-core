-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Transport of a concrete address is the identity, including when partially
-- applied under an array map in a generic vector search.
open LeanerIR.Proofs.Denote in
@[lir_denote_norm] theorem address_transport [Carriers] (θ : TypeArgs) :
    NTy.toSkolem θ .address = id := rfl

-- Generic helper calls also transport their parameter values by identity.
open LeanerIR.Proofs.Denote in
@[lir_denote_norm] theorem parameter_transport [Carriers] (θ : TypeArgs) (index : Nat) :
    NTy.toSkolem θ (.param index) = id := rfl

verify acquire by
  all_goals simp_all
  all_goals grind

verify acquire_linear by
  all_goals simp_all
  all_goals grind

-- Instantiating a phantom parameter cannot merge two different resource
-- declarations. A write to one therefore preserves every slot of the other.
open LeanerIR.Proofs.Denote in
theorem set_distinct_struct {unit} [Θ : Skolems unit]
    (memory : Memory unit) {h k : LeanerIR.StructHandle}
    (a b f g xs ys : NRow) (key otherKey : LeanerIR.StorageKey)
    (value : Option ((⟨Θ.resolve (.struct h a f), xs⟩ : ResourceType).carrier unit))
    (different : k ≠ h) :
    memory.set ⟨Θ.resolve (.struct h a f), xs⟩ key value
      ⟨Θ.resolve (.struct k b g), ys⟩ otherKey =
    memory ⟨Θ.resolve (.struct k b g), ys⟩ otherKey := by
  apply Memory.set_other
  intro eq
  have same := congrArg (fun r : ResourceType => r.type.handle?) eq
  simp only [NTy.handle?_resolve_struct, Option.some.injEq] at same
  exact different same

verify delegate by
  all_goals try simp (disch := decide) [set_distinct_struct,
    LeanerIR.Proofs.Denote.Skolems.ofRuntime_toRuntime] at *
  all_goals grind

verify revoke by
  all_goals try simp (disch := decide) [set_distinct_struct,
    LeanerIR.Proofs.Denote.findIndex?_eq_some_iff,
    LeanerIR.Proofs.Denote.findIndex?_eq_none_iff] at *
  all_goals grind
