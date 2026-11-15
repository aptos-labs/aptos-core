-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.SliceProvenanceDecls

set_option Elab.async false

open Lean Meta Elab Tactic VerexIR VerexSL

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  let holder := "invariant arr[0].value <= arr[1].value"
  -- (a) Slice birth through open reference negatives
  expectCuts `slice_provenance.slice_birth_array_open_bad
    [(percent, handOver)]
    (← `(command| verify slice_provenance::slice_birth_array_open_bad))
  expectCuts `slice_provenance.slice_birth_reborrow_open_bad
    [(percent, handOver)]
    (← `(command| verify slice_provenance::slice_birth_reborrow_open_bad))
  -- (b) Enclosing `Holder` relational invariant across slice mutation negatives
  -- (`Percent` element invariants close automatically, pinning `Holder` directly.)
  expectCuts `slice_provenance.slice_holder_order_bad
    [(holder, mutationEnd)]
    (← `(command| verify slice_provenance::slice_holder_order_bad))
  expectCuts `slice_provenance.slice_holder_either_bad
    [(holder, mutationEnd)]
    (← `(command| verify slice_provenance::slice_holder_either_bad))
  expectCuts `slice_provenance.slice_holder_diff_len_bad
    [(holder, mutationEnd), (holder, mutationEnd)]
    (← `(command| verify slice_provenance::slice_holder_diff_len_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `slice_provenance.slice_reborrow_single_bad
    [(percent, mutationEnd)]
    (← `(command| verify slice_provenance::slice_reborrow_single_bad))
  expectCuts `slice_provenance.slice_slot_reborrow_bad
    [(percent, mutationEnd)]
    (← `(command| verify slice_provenance::slice_slot_reborrow_bad))
  -- (d) Slice settlement current-element check through open reference negatives
  expectCuts `slice_provenance.slice_settle_open_bad
    [(percent, mutationEnd)]
    (← `(command| verify slice_provenance::slice_settle_open_bad))
  expectUnestablished `slice_provenance.slice_exit_open_bad [percent]
    (← `(command| verify slice_provenance::slice_exit_open_bad))

open LoanDeathControls in
run_cmd do
  -- Every negative must also reject the contradiction script (no pruning).
  for (base, command) in [
      (`slice_provenance.slice_birth_array_open_bad,
        ← `(command| verify slice_provenance::slice_birth_array_open_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`slice_provenance.slice_birth_reborrow_open_bad,
        ← `(command| verify slice_provenance::slice_birth_reborrow_open_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`slice_provenance.slice_holder_order_bad,
        ← `(command| verify slice_provenance::slice_holder_order_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega))))] do
    expectScriptFails base command

open LoanDeathControls in
run_cmd do
  for (base, command) in [
      (`slice_provenance.slice_holder_either_bad,
        ← `(command| verify slice_provenance::slice_holder_either_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`slice_provenance.slice_holder_diff_len_bad,
        ← `(command| verify slice_provenance::slice_holder_diff_len_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`slice_provenance.slice_reborrow_single_bad,
        ← `(command| verify slice_provenance::slice_reborrow_single_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega))))] do
    expectScriptFails base command

open LoanDeathControls in
run_cmd do
  for (base, command) in [
      (`slice_provenance.slice_slot_reborrow_bad,
        ← `(command| verify slice_provenance::slice_slot_reborrow_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`slice_provenance.slice_settle_open_bad,
        ← `(command| verify slice_provenance::slice_settle_open_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`slice_provenance.slice_exit_open_bad,
        ← `(command| verify slice_provenance::slice_exit_open_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega))))] do
    expectScriptFails base command
