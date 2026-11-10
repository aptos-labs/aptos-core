-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A generic callable result instantiated at Unit has no returned slot.
The specialized frame must still prove the callable contract; ordinary
non-Unit results continue to use the generic theorem. -/

namespace VerexSL.Tests.GenericUnitResults

verex namespace unit_result where
  @[method_evidence = 2]
  fun apply {F} {R}(receiver : F, permit : Bool, method : Fn(F) -> R has Copy, Drop) -> R :=
    invoke(method, receiver)
  spec apply where
    pragma opaque
    requires permit
    ensures ensures_of<method>(receiver, result)

  fun empty(receiver : u64) -> Unit := ()
  spec empty where
    pragma opaque
    ensures true

  fun run(receiver : u64) -> Unit :=
    apply(receiver, receiver == 0, function[Fn(u64) -> Unit has Copy, Drop](empty))
  spec run where
    requires receiver == 0
    ensures true

  fun identity(receiver : u64) -> u64 := receiver
  spec identity where
    pragma opaque
    ensures result == receiver

  fun nonunit(receiver : u64) -> u64 :=
    apply(receiver, true, function[Fn(u64) -> u64 has Copy, Drop](identity))
  spec nonunit where
    ensures result == receiver

-- A Unit-returning mutable method must retain its visible receiver effect.
verex namespace mutable_unit_result where
  @[method_evidence = 1]
  fun apply {F} {R}(receiver : &mut F, method : Fn(&mut F) -> R has Copy, Drop) -> R :=
    invoke(method, receiver)
  spec apply where
    pragma opaque
    ensures ensures_of<method>(old(receiver), result, receiver)
  fun set_seven(receiver : &mut u64) -> Unit := *receiver := 7
  spec set_seven where
    pragma opaque
    ensures receiver == 7
  fun run(receiver : &mut u64) -> Unit :=
    apply(receiver, function[Fn(&mut u64) -> Unit has Copy, Drop](set_seven))
  spec run where
    ensures receiver == 7

-- A lifetime's Unit placeholder must not force an incompatible-row instance.
-- Native shape contracts are selected by their own variables, not this frame.
verex namespace lifetime_unit_placeholder where
  native fun read {l : lifetime} {T}(value : &[l] T) -> T
  spec read at {u64} where
    ensures result == value
  fun run(value : &u64) -> u64 := read(value)
  spec run where
    ensures result == value

-- The specialized opaque contract must also discharge its scoped entry guard.
verex namespace scoped_unit_result where
  @[method_evidence = 1, scoped_method_evidence = 1]
  fun apply {F} {R}(receiver : &mut F, method : Fn(&mut F) -> R has Copy, Drop) -> R :=
    invoke(method, receiver)
  spec apply where
    pragma opaque
    ensures ensures_of<method>(old(receiver), result, receiver)
  fun set_seven(receiver : &mut u64) -> Unit := *receiver := 7
  spec set_seven where
    pragma opaque
    ensures receiver == 7
  fun run(receiver : &mut u64) -> Unit :=
    apply(receiver, function[Fn(&mut u64) -> Unit has Copy, Drop](set_seven))
  spec run where
    ensures receiver == 7

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  for name in [`VerexSL.Tests.GenericUnitResults.unit_result.run.verified,
      `VerexSL.Tests.GenericUnitResults.unit_result.nonunit.verified,
      `VerexSL.Tests.GenericUnitResults.mutable_unit_result.run.verified,
      `VerexSL.Tests.GenericUnitResults.scoped_unit_result.run.verified,
      `VerexSL.Tests.GenericUnitResults.lifetime_unit_placeholder.run.verified] do
    unless env.contains name do
      throwError "generic callable result did not verify: {name}"

-- Native erasure of a shared Unit reference must not erase its returned slot.
open Lean Elab Command VerexIR.Proofs.Denote in
run_cmd do
  let env ← getEnv
  let source := "verex namespace shared_unit_result where\n" ++
    "  fun accept(method : Fn(&Unit) -> &Unit has Copy, Drop) -> Unit := ()\n"
  let .ok stx := Parser.runParserCategory env `command source
    | throwError "shared Unit fixture did not parse"
  let .ok parsed := VerexSL.compilationUnitOfSyntax stx "shared-unit-result"
    | throwError "shared Unit fixture did not elaborate"
  let .ok unit := VerexSL.compileUnit parsed
    | throwError "shared Unit fixture did not validate"
  let some index := unit.tables.types.findIdx? (· matches .function ..)
    | throwError "shared Unit callable type missing"
  let some (.function _ sharing results) := ntyOf unit ⟨0⟩ ⟨index⟩
    | throwError "shared Unit callable has no native reading"
  unless results == .cons .unit .nil && sharing.results == [true] do
    throwError "shared Unit result lost its slot or sharing mark"

-- The independently checked specialization must keep the opaque contract boundary.
/--
info: `apply` uses concrete verification: Given interpretation does not yet support recursive or function-value call interpretation
---
error: the precondition `requires permit` of `apply` does not hold at call 1
---
error: verex verification failed: the automatic verification of `run` failed; provide a proof in the module:
  verify run by lean =>
    verex_obligations
    verex_on
    | requires_1_of_apply_at_call_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verex namespace missing_unit_precondition where
  @[method_evidence = 2]
  fun apply {F} {R}(receiver : F, permit : Bool, method : Fn(F) -> R has Copy, Drop) -> R :=
    invoke(method, receiver)
  spec apply where
    pragma opaque
    requires permit
    ensures ensures_of<method>(receiver, result)

  fun empty(receiver : u64) -> Unit := ()
  spec empty where
    pragma opaque
    ensures true

  fun run(receiver : u64) -> Unit :=
    apply(receiver, receiver == 0, function[Fn(u64) -> Unit has Copy, Drop](empty))
  spec run where
    ensures true

-- Intentionally omit the scoped receipt while retaining the behavioral contract.
/-- error: VIR-SCOPED-METHOD-EVIDENCE: a scoped method dictionary requires a checked literal or an unmodified scoped dictionary parameter; scoped requirements of a closure target must be captured -/
#guard_msgs in
verex namespace missing_unit_scope where
  @[method_evidence = 1, scoped_method_evidence = 1]
  fun apply {F} {R}(receiver : &mut F, method : Fn(&mut F) -> R has Copy, Drop) -> R :=
    invoke(method, receiver)
  spec apply where
    pragma opaque
    ensures ensures_of<method>(old(receiver), result, receiver)
  @[method_evidence = 1]
  fun run {F}(receiver : &mut F, method : Fn(&mut F) -> Unit has Copy, Drop) -> Unit :=
    apply(receiver, method)
  spec run where
    ensures true

end VerexSL.Tests.GenericUnitResults
