-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Behavioral predicates in contracts

A contract stating `aborts_of`, `ensures_of`, or `result_of` reads the
meaning of a function value from the executable unit
(`LeanerIR.Proofs.Behavior`): it takes the unit as its last parameter, and a
caller's theorem instantiates it at the unit it quantifies. Of a closure
whose target a caller sees, `requires_of` is the target's declared
precondition, and `ensures_of` and `aborts_of` what the target's theorem
states of the run.
-/

namespace LeanerLang.Tests.Behavior

open Lean Elab Command

leaner module 0x42::behavior where
  fun add(x : u64, y : u64) -> u64 := x + y

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)
    ensures result == result_of<f>(x)

  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](add, 1, _), x)
  spec use_apply where
    ensures true

run_cmd do
  let env ← getEnv
  let some contract := env.find? `«0x42».behavior.apply.typedInterfaceContract
    | throwError "the higher-order function's contract was not generated"
  let rec takesUnit : Lean.Expr → Bool
    | .forallE _ domain body _ =>
        domain.isAppOf ``LeanerIR.Validation.ExecutableUnit || takesUnit body
    | _ => false
  unless takesUnit contract.type do
    throwError "a contract stating behavioral predicates does not take the unit"
  let constants := contract.value!.getUsedConstants
  for predicate in [``LeanerIR.Proofs.AbortsOf, ``LeanerIR.Proofs.EnsuresOf,
      ``LeanerIR.Proofs.ResultOf] do
    unless constants.contains predicate do
      throwError s!"the contract does not state {predicate}"
  unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».behavior.use_apply.verified) do
    throwError "the caller of the higher-order function did not verify"

leaner module 0x42::dispatch where
  fun bounded(limit : u64, x : u64) -> u64 := x
  spec bounded where
    requires x <= limit
    ensures result == x

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    pragma verify = false
    requires requires_of<f>(x)
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](bounded, 10, _), x)
  spec use_apply where
    requires x <= 10
    ensures result == x

run_cmd do
  let env ← getEnv
  unless env.contains `«0x42».dispatch.requiresTable.eq_0 do
    throwError "the closure's target has no entry in the table of declared preconditions"
  unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».dispatch.use_apply.verified) do
    throwError "the caller did not verify through the target's precondition and theorem"

-- A caller that does not establish the target's precondition does not
-- establish `requires_of`.
/--
error: the precondition of `apply` is not established
---
error: leaner verification failed: the automatic verification of `use_apply` failed; provide a proof: `verify use_apply by …` in the module (`verify use_apply by skip` shows the obligations it leaves)
-/
#guard_msgs in
leaner module 0x42::unbounded where
  fun bounded(limit : u64, x : u64) -> u64 := x
  spec bounded where
    requires x <= limit
    ensures result == x

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    pragma verify = false
    requires requires_of<f>(x)

  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](bounded, 10, _), x)
  spec use_apply where
    ensures true

-- A higher-order function verifies against the predicates of the closure
-- it invokes: its theorem assumes the closure and global memory typed, and
-- the natives preserving typing and agreeing up to the loans they mint.
leaner module 0x42::unseen where
  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    modifies *
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)
    ensures result == result_of<f>(x)

run_cmd do
  let env ← getEnv
  let some verified := env.find? (`LeanerLang.Tests.Behavior ++ `«0x42».unseen.apply.verified)
    | throwError "the higher-order function did not verify"
  let constants := verified.type.getUsedConstants
  for assumption in [``LeanerIR.NativesTyped, ``LeanerIR.NativesShift] do
    unless constants.contains assumption do
      throwError s!"the theorem does not assume {assumption}"

-- A caller of such a function establishes what it assumes: a literal
-- closure is typed by evaluation over the unit, and global memory by the
-- caller's own assumption, which it takes in turn.
leaner module 0x42::callers where
  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > 18446744073709551615
    ensures result == x + y

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    modifies *
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](add, 1, _), x)
  spec use_apply where
    modifies *
    ensures true

run_cmd do
  let env ← getEnv
  let some verified := env.find? (`LeanerLang.Tests.Behavior ++ `«0x42».callers.use_apply.verified)
    | throwError "the caller of the higher-order function did not verify"
  unless verified.type.getUsedConstants.contains ``LeanerIR.NativesTyped do
    throwError "the caller does not take the natives' typing in turn"

-- Global memory stays typed through the function's own global operations:
-- a closure invoked after a publication, a removal, or a write through a
-- global borrow runs from typed memory.
leaner module 0x42::stateful where
  struct Counter has Key where
    value : u64

  fun publish_then_apply(account : &Signer, f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := do
    move_to<Counter>(account, new Counter { value := x })
    invoke(f, x)
  spec publish_then_apply where
    modifies *
    ensures true

  fun take_then_apply(addr : Address, f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := do
    let Counter { value := value } := move_from<Counter>(addr)
    invoke(f, value)
  spec take_then_apply where
    modifies *
    ensures true

  fun bump_then_apply(addr : Address, f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := do
    let value := &mut Counter[addr].value
    *value := x
    invoke(f, x)
  spec bump_then_apply where
    modifies *
    ensures true

run_cmd do
  let env ← getEnv
  for function in [`publish_then_apply, `take_then_apply, `bump_then_apply] do
    unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».stateful ++ function ++ `verified) do
      throwError s!"{function} did not verify"

-- A closure's target declared after the function passing it is verified
-- first, and a run the target returns from rules out the conditions under
-- which it must abort.
leaner module 0x42::later where
  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    modifies *
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](checked), x)
  spec use_apply where
    modifies *
    aborts_if x == 0
    ensures result == x

  fun checked(y : u64) -> u64 := if y == 0 then abort(1) else y
  spec checked where
    aborts_if y == 0
    ensures result == y

run_cmd do
  let env ← getEnv
  unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».later.use_apply.verified) do
    throwError "the caller did not verify through its later closure target"

-- A recursive specification function stating a behavioral predicate is
-- defined over the executable unit and the state its callers evaluate it in.
leaner module 0x42::iterated where
  fun add(x : u64, y : u64) -> u64 := x + y

  spec fun apply_n(f : Fn(u64) -> u64 has Copy, Drop, x : Int, n : Int) : Int :=
    if n <= 0 then x else result_of<f>(apply_n(f, x, n - 1))

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    pragma verify = false
    ensures result == apply_n(f, x, 1)

  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](add, 1, _), x)
  spec use_apply where
    ensures true

run_cmd do
  let env ← getEnv
  let some definition := env.find? `«0x42».iterated.apply_n.spec
    | throwError "the recursive specification function was not defined"
  let constants := definition.type.getUsedConstants
  for parameter in [``LeanerIR.Validation.ExecutableUnit, ``LeanerIR.Proofs.Denote.Memory] do
    unless constants.contains parameter do
      throwError s!"the definition does not take {parameter}"
  unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».iterated.use_apply.verified) do
    throwError "the caller did not verify"

-- A contract may name a function value by its target: the closure the
-- denotation builds of it, which the caller's proof dispatches to the
-- target's theorem.
leaner module 0x42::named where
  fun checked(y : u64) -> u64 := if y == 0 then abort(1) else y
  spec checked where
    aborts_if y == 0
    ensures result == y

  fun run(x : u64) -> u64 := checked(x)
  spec run where
    pragma opaque
    pragma verify = false
    ensures ensures_of<function[Fn(u64) -> u64 has Copy, Drop](checked)>(x, result)

  fun use_run(x : u64) -> u64 := run(x)
  spec use_run where
    ensures x != 0 ==> result == x

run_cmd do
  let env ← getEnv
  unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».named.use_run.verified) do
    throwError "the caller did not verify through the named function value"

-- A call of a function is a run of the function value naming it: a caller
-- establishes `ensures_of` of that value by calling the function.
leaner module 0x42::known where
  fun double(x : u64) -> u64 := x * 2
  spec double where
    aborts_if x * 2 > MAX_U64
    ensures result == x * 2

  fun call_double(x : u64) -> u64 := double(x)
  spec call_double where
    ensures ensures_of<function[Fn(u64) -> u64 has Copy, Drop](double)>(x, result)

  -- From typed memory the call is also the only run: no abort, and the
  -- result `result_of` reads.
  fun call_double_exactly(x : u64) -> u64 := double(x)
  spec call_double_exactly where
    aborts_if aborts_of<function[Fn(u64) -> u64 has Copy, Drop](double)>(x)
    ensures result == result_of<function[Fn(u64) -> u64 has Copy, Drop](double)>(x)

  -- A function returning nothing names a function value the same way.
  struct Counter has Key where
    value : u64

  fun bump(addr : Address) -> Unit := do
    let c := &mut Counter[addr].value
    *c := *c + 1
  spec bump where
    aborts_if !exists<Counter>(addr)
    aborts_if global<Counter>(addr).value + 1 > MAX_U64
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    modifies global<Counter>(addr)

  fun bump_once(addr : Address) -> Unit := bump(addr)
  spec bump_once where
    aborts_if aborts_of<function[Fn(Address) -> Unit has Copy, Drop](bump)>(addr)
    ensures ensures_of<function[Fn(Address) -> Unit has Copy, Drop](bump)>(addr)
    modifies global<Counter>(addr)

run_cmd do
  let env ← getEnv
  for function in [`call_double, `call_double_exactly, `bump_once] do
    unless env.contains (`LeanerLang.Tests.Behavior ++ `«0x42».known ++ function ++ `verified) do
      throwError s!"{function} did not establish the named function value's behavior"

-- The abort alternatives are marked by the opaque callee's specification.
-- On the second alternative, the first call must either overflow itself or
-- return x + 1 before the second invocation can be analyzed.
set_option leaner.verifyHeartbeats 50000 in
leaner module 0x42::nested_results where
  fun twice(f : Fn(u64) -> u64 has Copy, x : u64) -> u64 := invoke(f, invoke(f, x))
  spec twice where
    pragma opaque
    aborts_if aborts_of<f>(x) || aborts_of<f>(result_of<f>(x))
    ensures result == result_of<f>(result_of<f>(x))

  fun increment(x : u64) -> u64 := x + 1
  spec increment where
    aborts_if x + 1 > MAX_U64
    ensures result == x + 1

  fun add_two(x : u64) -> u64 :=
    twice(function[Fn(u64) -> u64 has Copy](increment), x)
  spec add_two where
    aborts_if x + 2 > MAX_U64
    ensures result == x + 2

-- Overflow of the second invocation must not be forgotten. At MAX_U64 - 1
-- the first invocation succeeds and the second aborts, refuting this clause.
/--
error: the specification clause `aborts_if x + 1 > MAX_U64` is not established
---
error: leaner verification failed: the automatic verification of `add_two` failed; provide a proof: `verify add_two by …` in the module (`verify add_two by skip` shows the obligations it leaves)
-/
#guard_msgs in
set_option leaner.verifyHeartbeats 50000 in
leaner module 0x42::wrong_nested_abort where
  fun twice(f : Fn(u64) -> u64 has Copy, x : u64) -> u64 := invoke(f, invoke(f, x))
  spec twice where
    pragma opaque
    aborts_if aborts_of<f>(x) || aborts_of<f>(result_of<f>(x))
    ensures result == result_of<f>(result_of<f>(x))

  fun increment(x : u64) -> u64 := x + 1
  spec increment where
    aborts_if x + 1 > MAX_U64
    ensures result == x + 1

  fun add_two(x : u64) -> u64 :=
    twice(function[Fn(u64) -> u64 has Copy](increment), x)
  spec add_two where
    aborts_if x + 1 > MAX_U64
    ensures result == x + 2

end LeanerLang.Tests.Behavior
