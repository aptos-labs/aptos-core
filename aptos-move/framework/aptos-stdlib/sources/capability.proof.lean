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
