-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# A returned mutable reference at a type with a data invariant

A behavioral predicate reads a returned mutable reference as its referent's
value at return, under the prophecy the run returns: a valid value, whose
current is the one the clause names.  The prophecy a call returned is such a
value, so the caller of the target establishes the statement; a clause that
names another current does not.

Of a target with type arguments the predicate is read directly of the target
and states that quantifier itself (`settledResult`): the prophecy's fact is
the first conjunct under it.  The statement is what this control pins there,
on a clause that names the wrong current.
-/

namespace VerexSL.Tests.Check.Closures.SettledResultValidity

verex module 0x42::settled_result_validity where
  struct Level has Copy, Drop where
    value : u64
  spec Level where
    invariant value <= 100

  fun raise(c : &mut Level) -> &mut Level := do
    c.value := c.value + 1
    c
  spec raise where
    requires c.value < 100
    aborts_if false
    ensures c.value == old(c).value + 1

  fun lift(c : &mut Level) -> Unit := do
    raise(c)
  spec lift where
    requires c.value < 100
    aborts_if false
    ensures ensures_of<function[Fn(&mut Level) -> &mut Level](raise)>(
      old(c), result_of<function[Fn(&mut Level) -> &mut Level](raise)>(old(c)), c)
    ensures result_of<function[Fn(&mut Level) -> &mut Level](raise)>(old(c)) == c

  fun lift_unchanged(c : &mut Level) -> Unit := do
    raise(c)
  spec lift_unchanged where
    requires c.value < 100
    ensures ensures_of<function[Fn(&mut Level) -> &mut Level](raise)>(
      old(c), old(c), c) -- error: the result is the raised level

  -- A target with a type argument: the predicate is read directly of it.
  fun raise_by {T has Copy, Drop}(c : &mut Level, step : T) -> &mut Level := do
    c.value := c.value + 1
    c
  spec raise_by where
    requires c.value < 100
    aborts_if false
    ensures c.value == old(c).value + 1

  fun lift_by_unchanged(c : &mut Level) -> Unit := do
    raise_by::<u64>(c, 0)
  spec lift_by_unchanged where
    requires c.value < 100
    ensures ensures_of<function[Fn(&mut Level, u64) -> &mut Level](raise_by::<u64>)>(
      old(c), 0, old(c), c) -- error: the result is the raised level

end VerexSL.Tests.Check.Closures.SettledResultValidity

/-! The statement `lift_by_unchanged` ensures of the target read directly
quantifies the prophecy with its fact first: an existential whose body is the conjunction of
`NTy.Holds` of the bound value with the rest.  The statement `lift` ensures
is the predicate's own (`EnsuresOfSettled`), which carries the same guard by
its definition. -/

open Lean Meta in
run_meta do
  let environment ← getEnv
  let states (function : String) (holds : Lean.Expr → Bool) : Bool :=
    environment.constants.map₂.toList.any fun (name, info) =>
      (name.toString.splitOn s!"settled_result_validity.{function}.").length == 2 &&
        (info.value?.bind fun value => value.find? holds).isSome
  let guarded (term : Lean.Expr) : Bool :=
    term.isAppOfArity ``Exists 2 &&
      (match term.getArg! 1 with
        | .lam _ _ body _ =>
            body.isAppOfArity ``And 2 &&
              (body.getArg! 0).isAppOfArity ``VerexIR.Proofs.Denote.NTy.Holds 3 &&
              (body.getArg! 0).getArg! 2 == .bvar 0
        | _ => false)
  unless states "lift_by_unchanged" guarded do
    throwError "the statement of `lift_by_unchanged` quantifies no valid prophecy"
  unless states "lift" (·.isAppOf ``VerexIR.Proofs.EnsuresOfSettled) do
    throwError "the statement of `lift` is not the predicate's own"
