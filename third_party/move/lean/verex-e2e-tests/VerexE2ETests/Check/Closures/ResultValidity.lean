-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The validity of an invocation's result

A function returning the result of an invocation owes the result's data
invariant. Of a function value it cannot see, that fact is assumed where its
result type states an invariant and discharged where a caller passes a value:
from the result type's own syntax where it states none, and from the caller's
own assumption for a value passed through. A result type that states no
invariant assumes nothing.
-/

namespace VerexSL.Tests.Check.Closures.ResultValidity

verex module 0x99::result_validity where
  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  fun apply {R}(f : Fn(u64) -> R, x : u64) -> R := invoke(f, x)
  spec apply where
    ensures ensures_of<f>(x, result)

  -- Passed through to another higher-order function, at the caller's own
  -- type parameter.
  fun apply_through {R}(f : Fn(u64) -> R, x : u64) -> R := apply(f, x)
  spec apply_through where
    ensures ensures_of<f>(x, result)

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result == new Percent { value := 50 }

  fun apply_fifty(x : u64) -> Percent := apply(function[Fn(u64) -> Percent](fifty), x)
  spec apply_fifty where
    ensures result.value == 50

  -- A claim about the result that neither the closure's behavior nor the
  -- result's invariant gives.
  fun apply_fifty_incorrect(x : u64) -> Percent :=
    apply(function[Fn(u64) -> Percent](fifty), x)
  spec apply_fifty_incorrect where
    ensures result.value == 40 -- error: the closure returns fifty

  -- A result type that states no invariant: nothing is assumed of the value.
  fun apply_plain(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec apply_plain where
    ensures ensures_of<f>(x, result)

  -- A higher-order function its callers reach through its contract, so that
  -- the fact is owed at the call that passes the value.
  fun apply_opaque {R}(f : Fn(u64) -> R, x : u64) -> R := invoke(f, x)
  spec apply_opaque where
    pragma opaque
    pragma aborts_if_is_partial
    ensures ensures_of<f>(x, result)

  fun double(x : u64) -> u64 := x * 2
  spec double where
    aborts_if x * 2 > MAX_U64
    ensures result == x * 2

  -- The fact of a closure whose result type states no invariant, owed at the
  -- call that passes the value. The call owes the callee's frame of the value
  -- as well, which is not established of a literal closure at the frame a
  -- generic callee's type arguments induce; the fact of its results is
  -- (`closureValidResults?`).
  fun apply_opaque_double(x : u64) -> u64 :=
    apply_opaque(function[Fn(u64) -> u64](double), x)
  spec apply_opaque_double where
    pragma aborts_if_is_partial
    ensures result == x * 2 -- error: the frame of the value is not established

  -- The same call where the callee's result type is concrete, so that no fact
  -- of the value's results is owed at all.
  fun apply_opaque_u64(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec apply_opaque_u64 where
    pragma opaque
    pragma aborts_if_is_partial
    ensures ensures_of<f>(x, result)

  fun apply_opaque_u64_double(x : u64) -> u64 :=
    apply_opaque_u64(function[Fn(u64) -> u64](double), x)
  spec apply_opaque_u64_double where
    pragma aborts_if_is_partial
    ensures result == x * 2

  -- A function whose results are valid only at arguments it requires.
  fun at_most(x : u64) -> Percent := new Percent { value := x }
  spec at_most where
    requires x <= 100
    aborts_if false
    ensures result.value == x

  -- An opaque higher-order function at a concrete result type that states an
  -- invariant, whose frame leaves every invocation of its parameter free: a
  -- caller owes of the value it passes only that its invocations return
  -- valid results. Its own body is a separate obligation, reported where it
  -- is verified: the unseen-invocation rule is stated for rows of scalar
  -- types (`ScalarAt`), which a declared result type is not.
  fun apply_opaque_percent(f : Fn(u64) -> Percent, x : u64) -> Percent :=
    invoke(f, x) -- error: the rule for an unseen invocation needs a scalar result row
  spec apply_opaque_percent where
    pragma opaque
    pragma aborts_if_is_partial
    modifies_of<f> *
    ensures ensures_of<f>(x, result)

  -- The caller cannot justify what it passes: the results of `at_most` are
  -- valid only under its precondition.
  fun apply_opaque_at_most(x : u64) -> Percent :=
    apply_opaque_percent(function[Fn(u64) -> Percent](at_most), x)
  spec apply_opaque_at_most where
    pragma aborts_if_is_partial -- error: the results of `at_most` are not valid at every argument

  -- The same call with a total function: the fact comes from its target's
  -- theorem.
  fun apply_opaque_fifty(x : u64) -> Percent :=
    apply_opaque_percent(function[Fn(u64) -> Percent](fifty), x)
  spec apply_opaque_fifty where
    pragma aborts_if_is_partial

open Lean Lean.Meta

-- The conjuncts a target's generated contract assumes of its arguments,
-- printed from the assumption reduced to them.
private def assumedConjuncts (base target : Name) : MetaM String := do
  let env ← getEnv
  let some name := [`typedContract, `contract].find? (fun suffix =>
      env.contains (base ++ target ++ suffix)) |>.map (base ++ target ++ ·)
    | throwError "{base ++ target} has no generated contract"
  let info ← getConstInfo name
  forallTelescope info.type fun binders _ => do
    let contract := mkAppN (mkConst name (info.levelParams.map Level.param)) binders
    let some unfolded ← unfoldDefinition? contract
      | throwError "{name} does not unfold to a contract"
    let assumes ← mkAppM ``VerexIR.Proofs.Contract.assumes #[unfolded]
    forallTelescope (← whnf (← inferType assumes)) fun arguments _ => do
      -- The projection and the application are reduced, the conjuncts are
      -- not: they are the facts the assumption states.
      let reduced ← whnfCore (mkAppN assumes arguments)
      pure (toString (← ppExpr reduced))

-- A function value is assumed to return valid results exactly where its
-- result type states an invariant: the fact stands among the conjuncts of a
-- generic result's assumption and among none of an invariant-free one's.
run_cmd Lean.Elab.Command.liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  let base := if env.contains (curNs ++ `«0x99».result_validity.apply_plain.typedContract) then
      curNs ++ `«0x99».result_validity
    else `«0x99».result_validity
  let states (printed fact : String) : Bool := (printed.splitOn fact).length > 1
  let generic ← assumedConjuncts base `apply
  unless states generic "KeepsMemoryAt" do
    throwError "a generic result's assumption is not reduced to its conjuncts:\n{generic}"
  unless states generic "ValidResultsAt" do
    throwError "a generic result does not assume the validity of its results:\n{generic}"
  let plain ← assumedConjuncts base `apply_plain
  unless states plain "KeepsMemoryAt" do
    throwError "an invariant-free result's assumption is not reduced to its conjuncts:\n{plain}"
  if states plain "ValidResultsAt" then
    throwError "an invariant-free result assumes the validity of its results:\n{plain}"

end VerexSL.Tests.Check.Closures.ResultValidity
