-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A generic result can carry a receiver loan. Dictionary metadata alone does
-- not establish that the receiver borrow ends when invocation returns.
import VerexE2ETests.CheckSupport
import VerexRust

verex namespace receiver_escape where
  @[method_evidence = 1]
  fun consume {F} {R}(mut owned : F, callback : Fn(&mut F) -> R has Copy, Drop) -> R := do
    let result := invoke(callback, &mut owned)
    drop(owned)
    result

  fun identity(value : &mut u64) -> &mut u64 := value

  fun dangling(value : u64) -> &mut u64 :=
    consume(value, function[Fn(&mut u64) -> &mut u64 has Copy, Drop](identity))

open Lean Elab Command in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `receiver_escape
    | throwError "missing test unit"
  match VerexIR.Validation.prepareExecution #[VerexIR.Rust.semantics] unit with
  | .ok _ => throwError "an invoked callback may return a borrow of its receiver"
  | .error errors =>
      unless errors.any (·.code == "VIR-SEMANTIC-BORROW-CONFLICT") do
        throwError "expected receiver borrow conflict, got {repr errors}"
