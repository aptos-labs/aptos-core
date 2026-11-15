-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

set_option maxHeartbeats 180000
set_option verex.verifyHeartbeats 180000

verex namespace nested_reference_invariants where
  pragma verify = false
  struct Bounded where
    value : u64
  spec Bounded where
    invariant value < 10

  fun read(value : &mut &mut Bounded) -> u64 := (**value).value
  spec read where
    ensures result < 10
    panics_if false
  verify read

  fun set_valid(value : &mut &mut Bounded) -> Unit := do
    (**value).value := 7u64
  spec set_valid where
    ensures value.value == 7
    panics_if false
  verify set_valid

  fun set_invalid(value : &mut &mut Bounded) -> Unit := do
    (**value).value := 11u64
  spec set_invalid where
    panics_if false

/--
error: the specification clause `invariant value < 10` is not established
---
error: verex verification failed: the automatic verification of `set_invalid` failed; provide a proof in the module:
  verify set_invalid by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify nested_reference_invariants::set_invalid

verex namespace folded_boundary_and_loop_invariants where
  pragma verify = false
  struct MultiBounded where
    low : u64
    high : u64
  spec MultiBounded where
    invariant low < 10
    invariant high > 20

  struct Holder {T} where
    item : T

  fun noop_nested(value : &mut &mut MultiBounded, items : &mut &mut Vector<MultiBounded>) -> Unit :=
    ()
  spec noop_nested where
    panics_if false
  verify noop_nested

  fun loop_keep(value : &mut MultiBounded, limit : u64) -> Unit := do
    let mut i := 0u64
    while i < limit do
      (*value).low := 3u64
      (*value).high := 25u64
      i := i + 1u64
  spec loop_keep where
    panics_if false
  verify loop_keep

  fun break_second_boundary(value : &mut &mut MultiBounded) -> Unit := do
    (**value).low := 5u64
    (**value).high := 15u64
  spec break_second_boundary where
    panics_if false

  fun set_holder_valid(holder : Holder<&mut MultiBounded>) -> Unit := do
    (*holder.item).low := 4u64
    (*holder.item).high := 30u64
  spec set_holder_valid where
    panics_if false
  verify set_holder_valid

  fun break_holder(holder : Holder<&mut MultiBounded>) -> Unit := do
    (*holder.item).low := 12u64
  spec break_holder where
    panics_if false

  fun break_loop_entry(value : &mut MultiBounded, limit : u64) -> Unit := do
    (*value).high := 10u64
    let mut i := 0u64
    while i < limit do
      (*value).low := 3u64
      i := i + 1u64
  spec break_loop_entry where
    panics_if false

  fun break_loop_step(value : &mut MultiBounded, limit : u64) -> Unit := do
    let mut i := 0u64
    while i < limit do
      (*value).low := 15u64
      i := i + 1u64
  spec break_loop_step where
    panics_if false

  fun prove_boundary_routed(value : &mut &mut MultiBounded, next_high : u64) -> Unit := do
    (**value).low := 2u64
    (**value).high := next_high
  spec prove_boundary_routed where
    requires next_high > 21
    panics_if false

  fun prove_loop_routed(value : &mut MultiBounded, next_high : u64, limit : u64) -> Unit := do
    let mut i := 0u64
    while i < limit do
      (*value).low := 2u64
      (*value).high := next_high
      i := i + 1u64
  spec prove_loop_routed where
    requires next_high > 21
    panics_if false

-- (a) Boundary contracts and loop invariants emit folded `NTy.Holds` without
-- inline per-field/per-element expansions or outer `Obligation` range wrappers.
open Lean Meta Elab Command VerexSL VerexSL.Contract VerexSL.Verify VerexIR.Proofs.Denote in
run_cmd do
  let env ← getEnv
  let contract ← getConstInfo `folded_boundary_and_loop_invariants.noop_nested.typedContract
  let contractBody := contract.value?.getD contract.type
  unless (contractBody.find? (·.isConstOf ``NTy.Holds)).isSome do
    throwError "boundary contract did not emit folded NTy.Holds"
  if (contractBody.find? fun e =>
      e.isAppOf ``VerexIR.Proofs.Obligation && (e.find? (·.isConstOf ``NTy.Holds)).isSome).isSome then
    throwError "boundary NTy.Holds was wrapped in an outer Obligation marker"
  if (contractBody.find? (·.isConstOf ``Membership.mem)).isSome then
    throwError "nested &mut Vector<MultiBounded> expanded elementwise instead of emitting folded NTy.Holds"
  let segments := #["folded_boundary_and_loop_invariants"]
  let some unit := registeredUnit? env `folded_boundary_and_loop_invariants
    | throwError "missing folded_boundary_and_loop_invariants unit"
  let unitDef ← ensureUnitDefinition segments unit
  let own ← ensureDataOwn segments unit
  let ns := unit.namespaces[0]!
  let some fnIdx := ns.functions.findIdx? fun decl =>
      ((ns.nameOf decl.name).map (·.name)) == some "loop_keep"
    | throwError "missing loop_keep function"
  let decl := ns.functions[fnIdx]!
  let some view := compileNamespaceAt? unit ⟨0⟩ | throwError "missing compile view"
  let Except.ok compiled := compileFunction unit ⟨⟨0⟩, ⟨fnIdx⟩⟩
    | throwError "failed to compile loop_keep"
  liftTermElabM <| withNominalUnit unitDef do
    withExecutableSkolems unit fun executable skolems => do
      let types ← frameTypes skolems
      let loops ← loopInvariants unit executable ⟨0⟩ ns decl
        compiled.params compiled.frame types none
        (markedInvariant own view) (carried := own.carriers)
      let some (_, loopPred, _) := loops[0]? | throwError "missing generated loop invariant"
      unless (loopPred.find? (·.isConstOf ``NTy.Holds)).isSome do
        throwError "loop invariant did not emit folded NTy.Holds"
      if (loopPred.find? (·.isConstOf ``VerexIR.Proofs.Obligation)).isSome then
        throwError "loop NTy.Holds was wrapped in an outer Obligation marker"

-- (b) Multi-clause failure diagnostics report only the exact failing clause's
-- ObligationRange without duplicate struct-wide errors.
/--
error: the specification clause `invariant high > 20` is not established
---
error: verex verification failed: the automatic verification of `break_second_boundary` failed; provide a proof in the module:
  verify break_second_boundary by lean =>
    verex_obligations
    verex_on
    | data_invariant_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify folded_boundary_and_loop_invariants::break_second_boundary

/--
error: the specification clause `invariant low < 10` is not established
---
error: verex verification failed: the automatic verification of `break_holder` failed; provide a proof in the module:
  verify break_holder by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify folded_boundary_and_loop_invariants::break_holder

/--
error: the loop invariant `invariant high > 20` is not established at entry
---
error: verex verification failed: the automatic verification of `break_loop_entry` failed; provide a proof in the module:
  verify break_loop_entry by lean =>
    verex_obligations
    verex_on
    | data_invariant_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify folded_boundary_and_loop_invariants::break_loop_entry

/--
error: the loop invariant `invariant low < 10` is not preserved by an iteration
---
error: verex verification failed: the automatic verification of `break_loop_step` failed; provide a proof in the module:
  verify break_loop_step by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify folded_boundary_and_loop_invariants::break_loop_step

-- (c) Authored proof routing `on data invariant k of S` works on both boundary
-- and loop `NTy.Holds` obligations.
/--
trace: routed boundary clause 2
-/
#guard_msgs in
verify folded_boundary_and_loop_invariants::prove_boundary_routed by
  on data invariant 2 of MultiBounded {
    lean =>
      trace "routed boundary clause 2"
      verex_decide
  }
  on _ { }

/--
trace: routed loop clause 2
-/
#guard_msgs in
verify folded_boundary_and_loop_invariants::prove_loop_routed by
  on data invariant 2 of MultiBounded {
    lean =>
      trace "routed loop clause 2"
      verex_decide
  }
  on _ { }

run_cmd do
  let env ← Lean.getEnv
  for fn in [`prove_boundary_routed, `prove_loop_routed, `set_holder_valid, `loop_keep, `noop_nested] do
    unless env.contains (`folded_boundary_and_loop_invariants ++ fn ++ `verified) do
      throwError "missing verified theorem for {fn}"

-- (d) Generic `.param` loop and nested-reference obligations under `Given`:
-- opaque `NTy.Holds (.param 0)` closes directly across loop back-edges,
-- shared-continuation joins (where `assertCertifiedClauses` hides the entry
-- `NTy.Holds` as `.implDetail`), and `verex_prepare`.
section GivenParamLoop
set_option verex.givenRoute true

verex module 0x42::given_folded_param_loop where
  pragma verify = false

  fun replace_once_in_loop {T has Drop}(slot : &mut T, replacement : T, count : u64) -> Unit := do
    let mut i : u64 := 0
    while i < count do
      if i == 3 then
        *slot := replacement
        break
      i := i + 1
  spec replace_once_in_loop where
    aborts_if false

  fun push_in_loop {T has Drop}(items : &mut Vector<T>, value : T, flag : Bool) -> Unit := do
    let mut i : u64 := 0
    while i < 1 do
      if flag then
        *items := core.prim.pushVector(*items, value)
      break
  spec push_in_loop where
    aborts_if false

verify 0x42::given_folded_param_loop::replace_once_in_loop
verify 0x42::given_folded_param_loop::push_in_loop

verex namespace given_nested_param_ref where
  pragma verify = false

  struct Holder {T} where
    item : T

  fun keep_holder_param {T}(holder : Holder<&mut T>) -> Unit := ()
  spec keep_holder_param where
    panics_if false

  fun branch_nested_param {T}(value : &mut &mut T, flag : Bool) -> Unit := do
    let mut tag := 0u64
    if flag then
      tag := 1u64
    else
      tag := 2u64
    if tag > 0u64 then
      ()
  spec branch_nested_param where
    panics_if false

  fun prepare_nested_param {T}(value : &mut &mut T) -> Unit := ()
  spec prepare_nested_param where
    panics_if false

verify given_nested_param_ref::keep_holder_param
verify given_nested_param_ref::branch_nested_param
verify given_nested_param_ref::prepare_nested_param by lean =>
  verex_obligations
  all_goals (verex_prepare; verex_decide)

open Lean Elab Command VerexSL.Verify in
run_cmd do
  let env ← getEnv
  for fn in [`replace_once_in_loop, `push_in_loop] do
    let base := `«0x42».given_folded_param_loop ++ fn
    unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
      throwError "{fn} did not verify via the Given proof path"
  for fn in [`keep_holder_param, `branch_nested_param] do
    let base := `given_nested_param_ref ++ fn
    unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
      throwError "{fn} did not verify via the Given proof path"
  unless env.contains `given_nested_param_ref.prepare_nested_param.verified do
    throwError "missing verified theorem for prepare_nested_param"

end GivenParamLoop

-- (e) Pinning (`verex_pin_terms`) reading a bound carried inside a folded
-- `NTy.Holds` boundary certificate: positive controls (single-clause and
-- multi-clause folded `Holds`, asserting `step pin terms ×1 ok`) and negative
-- control without the invariant.
verex namespace folded_holds_pinning where
  pragma verify = false

  struct OrderedPair where
    low : u64
    high : u64
  spec OrderedPair where
    invariant low <= high

  struct BoundedOrderedPair where
    low : u64
    high : u64
  spec BoundedOrderedPair where
    invariant low <= high
    invariant high < 60

  struct UnorderedPair where
    low : u64
    high : u64

  fun square_from_folded_holds(pair : OrderedPair) -> u64 :=
    match pair with
      | OrderedPair { low := low, high := _ } => low
  spec square_from_folded_holds where
    requires pair.high <= pair.low
    ensures result * result == pair.high * pair.high
    panics_if false

  fun shift_from_multi_folded_holds(pair : BoundedOrderedPair) -> u64 :=
    match pair with
      | BoundedOrderedPair { low := low, high := _ } => 1u64 << (low as u8)
  spec shift_from_multi_folded_holds where
    requires pair.high <= pair.low
    ensures result * result ==
      ((1 << pair.high) % 18446744073709551616) * ((1 << pair.high) % 18446744073709551616)
    panics_if false

  fun square_without_holds(pair : UnorderedPair) -> u64 :=
    match pair with
      | UnorderedPair { low := low, high := _ } => low
  spec square_without_holds where
    requires pair.high <= pair.low
    ensures result * result == pair.high * pair.high
    panics_if false

open Lean Elab Command in
elab "#guard_pin_terms " command:command : command => do
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.solverProfile true in $command))
  unless (output.splitOn "step pin terms ×1 ok").length > 1 do
    throwError "expected `verex_pin_terms` to pin from folded `Holds`, got:\n{output}"

#guard_pin_terms verify folded_holds_pinning::square_from_folded_holds

verify folded_holds_pinning::shift_from_multi_folded_holds

/--
error: the specification clause `ensures result * result == pair.high * pair.high` is not established
---
error: verex verification failed: the automatic verification of `square_without_holds` failed; provide a proof in the module:
  verify square_without_holds by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify folded_holds_pinning::square_without_holds

run_cmd do
  let env ← Lean.getEnv
  for fn in [`square_from_folded_holds, `shift_from_multi_folded_holds] do
    unless env.contains (`folded_holds_pinning ++ fn ++ `verified) do
      throwError "missing verified theorem for {fn}"
  if env.contains `folded_holds_pinning.square_without_holds.verified then
    throwError "unexpected verified theorem for negative control square_without_holds"
