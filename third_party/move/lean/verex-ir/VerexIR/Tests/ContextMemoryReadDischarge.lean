-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base
import VerexIR.Proofs.Denote.Solve.Normal

open Lean Meta Elab Tactic VerexIR.Proofs.Denote

namespace ContextMemoryReadDischarge

-- When `verex_context_memory_reads` substitutes a storage read inside a hypothesis
-- and exposes a term governed by a conditional `vir_denote_norm` simproc
-- (`tdivOfNonnegative`), it must pass `contextDischarge` so context bounds discharge
-- the side condition before `resultsNormal := true` records the result in `readForms`.
private theorem memory_read_discharges_conditional_norm
    {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [s : Skolems unit]
    (m : Memory unit) (k : VerexIR.StorageKey) (fee fallback : NTy.num.carrier)
    (toInt : NTy.num.carrier → Int) (amount : Int)
    (heq : Skolems.plainAt NTy.num NRow.nil k m = some fee)
    (h_nonneg : 0 ≤ amount * (10000 - toInt fee))
    (h_quot :
      (amount * (10000 - toInt ((Skolems.plainAt NTy.num NRow.nil k m).getD fallback))).tdiv
        10000 = 0) :
    amount * (10000 - toInt fee) / 10000 = 0 := by
  verex_context_memory_reads
  exact h_quot

open Lean Elab Command in
run_cmd do
  for name in [``memory_read_discharges_conditional_norm] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end ContextMemoryReadDischarge
