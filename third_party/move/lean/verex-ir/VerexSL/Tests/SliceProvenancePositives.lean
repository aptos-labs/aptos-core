-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.SliceProvenanceDecls
import VerexSL.Tests.SliceLeafSupport

set_option Elab.async false

open Lean Meta Elab Tactic VerexIR VerexSL

verify slice_provenance::slice_birth_array_open_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_birth_reborrow_open_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_holder_order_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_holder_either_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_holder_diff_len_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_slot_reborrow_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_settle_open_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

verify slice_provenance::slice_exit_open_ok by lean =>
  verex_obligations
  all_goals verex_slice_close

run_cmd do
  let env ← getEnv
  for name in [
      `slice_provenance.slice_birth_array_open_ok.typedVerified,
      `slice_provenance.slice_birth_reborrow_open_ok.typedVerified,
      `slice_provenance.slice_holder_order_ok.typedVerified,
      `slice_provenance.slice_holder_either_ok.typedVerified,
      `slice_provenance.slice_holder_diff_len_ok.typedVerified,
      `slice_provenance.slice_slot_reborrow_ok.typedVerified,
      `slice_provenance.slice_settle_open_ok.typedVerified,
      `slice_provenance.slice_exit_open_ok.typedVerified] do
    unless env.contains name do throwError "the slice control {name} did not verify"
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"
