-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs
open VerexIR VerexIR.Proofs.Denote

private abbrev Entry : NTy := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.param 0) .nil)
private abbrev Bag : NTy := .struct ⟨⟨0⟩, 1⟩ .nil (.cons (.vector Entry) .nil)

example [family : Carriers] [CarriersNonempty family] (owner : Bag.carrier) :
    ∀ index : Nat, index < owner.val.fst.values.size →
      ((@Var.selectCertified family ⟨⟨0⟩, 1⟩ .nil (.cons (.vector Entry) .nil)
          (.vector Entry) (.here) owner).values[index]?.getD
            (NTy.inhabitant Entry (by rfl))).val =
        owner.val.fst.values[index]?.getD (NTy.plainInhabitant Entry (by rfl)) := by
  intro index bounded
  verex_normalize_context
  all_goals trivial
