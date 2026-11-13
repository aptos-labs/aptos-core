-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

namespace VerexSL.Tests.Check.Closures.FnOnceEvidence

verex namespace methods where
  struct Token has Drop where
    value : u64

  @[method_evidence = 2]
  fun apply_once {F} {A} {R}(callback : F, argument : A,
      method : Fn(F, A) -> R has Copy, Drop) -> R :=
    invoke(method, callback, argument)
  spec apply_once where
    pragma opaque
    ensures ensures_of<method>(callback, argument, result)

  @[method_evidence = 2]
  fun forward {F} {A} {R}(callback : F, argument : A,
      method : Fn(F, A) -> R has Copy, Drop) -> R :=
    apply_once(callback, argument, method)
  spec forward where
    pragma opaque
    ensures ensures_of<method>(callback, argument, result)

  fun consume(token : Token, n : u64) -> u64 := token.value + n
  spec consume where
    pragma opaque
    ensures result == (token.value + n) % 18446744073709551616

  fun count(n : u64) -> u64 :=
    forward(new Token { value := n }, 1u64, function[Fn(Token, u64) -> u64 has Copy, Drop](consume))
  spec count where
    ensures result == (n + 1) % 18446744073709551616

-- Opaque calls must reuse the generic proofs, including forwarding evidence.
open Lean Elab Command in
run_cmd do
  let base := `VerexSL.Tests.Check.Closures.FnOnceEvidence.methods
  let environment ← getEnv
  for (caller, callee) in [("count", "forward"), ("forward", "apply_once")] do
    let mut pending := [base ++ Name.mkSimple caller ++ `typedVerified]
    let mut visited : NameSet := {}
    let mut used : NameSet := {}
    while let next :: rest := pending do
      pending := rest
      if visited.contains next then continue
      visited := visited.insert next
      let some proof := environment.find? next |>.bind (·.value? (allowOpaque := true))
        | throwError "missing caller proof {next}"
      for constant in proof.getUsedConstants do
        used := used.insert constant
        if (base ++ Name.mkSimple caller).isPrefixOf constant then pending := constant :: pending
    unless used.contains (base ++ Name.mkSimple callee ++ `typedSemanticsVerified) do
      throwError "{caller} did not reuse the generic proof of {callee}"

end VerexSL.Tests.Check.Closures.FnOnceEvidence
