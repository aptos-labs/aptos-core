-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A generic callee used through its contract states that contract at the
caller's frame: the recursive specification functions its clauses apply are
transported there (`ensureDefinitionTransport`), so a caller's clause and the
hypothesis the call supplies are one term — at the caller's own type
parameters, where the transport is the definition itself, and at an instance
of them, where it is the definition at the substituted type argument. A
definition a condition keeps at the frame the type arguments induce is an
error at specialization, not a second form of the caller's frame. -/

set_option verex.verifyHeartbeats 400000

namespace VerexSL.Tests.SpecializedDefinitions

verex module 0x42::specialized_definitions where
  pragma verify = false

  -- Occurrences of `needle` among the positions `[lo, hi)`.
  spec fun count {T}(values : Vector<T>, needle : T, lo : Int, hi : Int) : Int decreases hi - lo :=
    if hi <= lo then 0
    else count(values, needle, lo, hi - 1) + (if values[hi - 1] == needle then 1 else 0)

  spec fun same_counts {T}(before : Vector<T>, after : Vector<T>, lo : Int, hi : Int) : Bool :=
    forall (x : T), count(before, x, lo, hi) == count(after, x, lo, hi)

  -- A specification function whose own body applies one that applies the
  -- recursive definition: the frame reaches `count` through two calls.
  spec fun counts_both_ways {T}(before : Vector<T>, after : Vector<T>, lo : Int,
      hi : Int) : Bool :=
    same_counts(before, after, lo, hi) && same_counts(after, before, lo, hi)

  -- Opaque: callers reason through the contract, which states the counts.
  fun keep {T has Copy, Drop, Store}(values : Vector<T>) -> Vector<T> := values
  spec keep where
    pragma opaque
    ensures result.length == values.length
    ensures counts_both_ways(values, result, 0, values.length)
    aborts_if false

  -- A call at the caller's own type parameters, in order.
  public fun keep_own {T has Copy, Drop, Store}(values : Vector<T>) -> Vector<T> :=
    keep::<T>(values)
  spec keep_own where
    ensures result.length == values.length
    ensures counts_both_ways(values, result, 0, values.length)
    aborts_if false

  -- A call at an instance of the caller's own type parameter.
  public fun keep_vectors {T has Copy, Drop, Store}(values : Vector<Vector<T> >) ->
      Vector<Vector<T> > := keep::<Vector<T> >(values)
  spec keep_vectors where
    ensures result.length == values.length
    ensures counts_both_ways(values, result, 0, values.length)
    aborts_if false

  -- A nested instance of the caller's own type parameter.
  public fun keep_nested {T has Copy, Drop, Store}(values : Vector<Vector<Vector<T> > >) ->
      Vector<Vector<Vector<T> > > := keep::<Vector<Vector<T> > >(values)
  spec keep_nested where
    ensures result.length == values.length
    ensures counts_both_ways(values, result, 0, values.length)
    aborts_if false

  -- A call at the caller's second type parameter: a row that is not the
  -- caller's parameters in order.
  public fun keep_second {T has Copy, Drop, Store} {U has Copy, Drop, Store}(
      values : Vector<U>) -> Vector<U> := keep::<U>(values)
  spec keep_second where
    ensures result.length == values.length
    ensures counts_both_ways(values, result, 0, values.length)
    aborts_if false

  -- A wrong claim about the helper's result: its counts are not one more.
  public fun keep_wrong {T has Copy, Drop, Store}(values : Vector<T>) -> Vector<T> :=
    keep::<T>(values)
  spec keep_wrong where
    ensures forall (x : T),
      count(values, x, 0, values.length) == count(result, x, 0, values.length) + 1
    aborts_if false

-- A wrong claim about the callee's result is not established by the
-- specialized contract.
/--
error: the specification clause `ensures forall (x : T),
      count(values, x, 0, values.length) == count(result, x, 0, values.length) + 1` is not established
---
error: verex verification failed: the automatic verification of `keep_wrong` failed; provide a proof in the module:
  verify keep_wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::specialized_definitions::keep_wrong

verify 0x42::specialized_definitions::keep
verify 0x42::specialized_definitions::keep_own
verify 0x42::specialized_definitions::keep_vectors
verify 0x42::specialized_definitions::keep_nested
verify 0x42::specialized_definitions::keep_second


-- Each call is specialized at its row, and the specification function the
-- callee's contract applies is transported to the caller's frame: at the
-- caller's own parameters the definition itself, at an instance the
-- definition at the substituted type argument.
open Lean Elab Command in
run_cmd do
  let env ← getEnv
  let member := `VerexSL.Tests.SpecializedDefinitions ++ `«0x42» ++ `specialized_definitions
  for name in [member ++ `keep ++ `verified, member ++ `keep_own ++ `verified,
      member ++ `keep_vectors ++ `verified, member ++ `keep_nested ++ `verified,
      member ++ `keep_second ++ `verified] do
    unless env.contains name do
      throwError "a specialized call did not verify: {name}"
  let module := `«0x42» ++ `specialized_definitions
  -- The recursive definition is transported at each row the calls take; the
  -- specification functions above it are not definitions of their own, so the
  -- frame reaches `count` through them.
  let generic := module ++ `count ++ `at_T0 ++ `spec
  for row in ["at_T0", "at_T1", "at_vector<T0>", "at_vector<vector<T0>>"] do
    let name := generic ++ Name.mkSimple ("transport_" ++ row)
    unless env.contains name do
      throwError "a specification function was not transported: {name}"
  for row in ["at_T1", "at_vector<T0>", "at_vector<vector<T0>>"] do
    let name := module ++ `count ++ Name.mkSimple row ++ `spec
    unless env.contains name do
      throwError "a specification function has no definition at its row: {name}"

-- The guard at the producer's boundary (`definitionAtInducedFrame?`): a
-- condition that keeps a generated definition at the frame the type arguments
-- induce is rejected, naming the condition, the contract, the row and the
-- frame; the same definition at the caller's own frame is accepted, and so is
-- a canonical application whose value argument carries an induced frame of its
-- own. The applications are built for the scan, which reads a definition's
-- frame arguments.
open Lean Meta Elab Command VerexIR.Proofs.Denote VerexSL.Verify in
run_cmd liftTermElabM do
  let definition := `«0x42» ++ `specialized_definitions ++ `count ++ `at_T0 ++ `spec
  let contract := `«0x42» ++ `specialized_definitions ++ `keep ++ `typedContract
  let some info := (← getEnv).find? definition
    | throwError "the recursive specification function is not defined: {definition}"
  forallTelescope info.type fun binders _ => do
    let unitExpr := binders[0]!
    let functions := binders[1]!
    let bundle := binders.back!
    withLocalDeclD `Θ (mkApp2 (mkConst ``Skolems) unitExpr functions) fun frame =>
    withLocalDeclD `θ (mkConst ``TypeArgs) fun θ => do
      let induced := mkApp4 (mkConst ``Skolems.instantiate) unitExpr functions θ frame
      let carriersOf (frame : Lean.Expr) :=
        mkApp3 (mkConst ``Skolems.toCarriers) unitExpr functions frame
      let typesOf (frame : Lean.Expr) :=
        mkApp3 (mkConst ``Skolems.type) unitExpr functions frame
      let applied (at_ : Lean.Expr) (value : Lean.Expr) :=
        mkAppN (mkConst definition)
          #[unitExpr, functions, carriersOf at_, typesOf at_, value]
      let some found ← definitionAtInducedFrame? `ensures (applied induced bundle)
        | throwError "the guard accepted a definition left at an induced frame"
      let reported ← (← addMessageContext
        (found.message contract (NRow.cons (NTy.param 0) NRow.nil))).toString
      let expected := "the condition `ensures` of the contract \
        `«0x42».specialized_definitions.keep.typedContract` applies the definition \
        `«0x42».specialized_definitions.count.at_T0.spec` at the frame the type \
        arguments `at_T0` induce, which has no transport: \
        VerexIR.Proofs.Denote.Skolems.instantiate\n  θ Θ"
      unless reported == expected do
        throwError "the guard reports{indentD reported}instead of{indentD expected}"
      for (case, term) in [("the caller's own frame", applied frame bundle),
          ("an induced frame in a value", applied frame (carriersOf induced))] do
        if (← definitionAtInducedFrame? `ensures term).isSome then
          throwError "the guard rejected a canonical application: {case}"


end VerexSL.Tests.SpecializedDefinitions
