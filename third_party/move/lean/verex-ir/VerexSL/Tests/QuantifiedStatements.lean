-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Proofs.Denote

namespace VerexSL.Tests.QuantifiedStatements

-- Reading membership builds the equivalent quantified statement with the
-- array element's plain type. Its carrier type is definitionally the same.
example [Carriers] (values : (NTy.vector .address).plain) (address : NTy.address.carrier)
    (absent : ∀ i : Int, 0 ≤ i → i < values.values.size →
      ¬values.values[i.toNat]? = some address) : address ∉ values.values := by
  verex_block_decide

example [Carriers] (values : (NTy.vector .address).plain) (address : NTy.address.carrier)
    (absent : ∀ i : Int, 0 ≤ i → i < values.values.size →
      values.values[i.toNat]? ≠ some address) : address ∉ values.values := by
  verex_block_decide

-- A different element's absence does not establish the requested statement.
set_option linter.unusedVariables false in
example [Carriers] (values : (NTy.vector .address).plain) (address other : NTy.address.carrier)
    (absent : ∀ i : Int, 0 ≤ i → i < values.values.size →
      ¬values.values[i.toNat]? = some other) : True := by
  fail_if_success
    have : address ∉ values.values := by verex_block_decide
  trivial

end VerexSL.Tests.QuantifiedStatements
