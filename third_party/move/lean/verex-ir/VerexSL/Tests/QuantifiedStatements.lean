-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Proofs.Denote

namespace VerexSL.Tests.QuantifiedStatements

-- A different element's absence does not establish the requested statement.
set_option linter.unusedVariables false in
example [Carriers] (values : (NTy.vector .address).carrier) (address other : NTy.address.carrier)
    (absent : ∀ i : Int, 0 ≤ i → i < values.values.size →
      ¬values.values[i.toNat]? = some other) : True := by
  fail_if_success
    have : address ∉ values.values := by verex_block_decide
  trivial

-- Reading membership builds the equivalent quantified statement with the
-- array element's carrier type.
example [Carriers] (values : (NTy.vector .address).carrier) (address : NTy.address.carrier)
    (absent : ∀ i : Int, 0 ≤ i → i < values.values.size →
      ¬values.values[i.toNat]? = some address) : address ∉ values.values := by
  verex_block_decide

example [Carriers] (values : (NTy.vector .address).carrier) (address : NTy.address.carrier)
    (absent : ∀ i : Int, 0 ≤ i → i < values.values.size →
      values.values[i.toNat]? ≠ some address) : address ∉ values.values := by
  verex_block_decide

end VerexSL.Tests.QuantifiedStatements
