-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

namespace VerexSL.Tests.Check.Closures.FnMutEvidence

-- A scalar result must preserve both the receiver transition and return value.
verex namespace methods where
  @[method_evidence = 1]
  fun apply_mut {F}(callback : &mut F, method : Fn(&mut F) -> u64 has Copy, Drop) -> u64 := do
    invoke(method, &mut *callback)
    invoke(method, &mut *callback)
  spec apply_mut where
    pragma opaque
    ensures exists (middle : F), exists (first : Int),
      ensures_of<method>(old(callback), first, middle) &&
      ensures_of<method>(middle, result, callback)

  @[method_evidence = 1]
  fun forward {F}(callback : &mut F, method : Fn(&mut F) -> u64 has Copy, Drop) -> u64 :=
    apply_mut(callback, method)
  spec forward where
    pragma opaque
    ensures exists (middle : F), exists (first : Int),
      ensures_of<method>(old(callback), first, middle) &&
      ensures_of<method>(middle, result, callback)

  fun bump(value : &mut u64) -> u64 := do
    *value := *value + 1
    *value
  spec bump where
    pragma opaque
    ensures value == (old(value) + 1) % 18446744073709551616
    ensures result == value

  fun count(n : u64) -> u64 := do
    let mut value := n
    forward(&mut value, function[Fn(&mut u64) -> u64 has Copy, Drop](bump))
  spec count where
    ensures result == (n + 2) % 18446744073709551616

-- Opaque calls must reuse the generic proofs, including forwarding evidence.
open Lean Elab Command in
run_cmd do
  let base := `VerexSL.Tests.Check.Closures.FnMutEvidence.methods
  let environment ← getEnv
  for (caller, callee) in [("count", "forward"), ("forward", "apply_mut")] do
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

end VerexSL.Tests.Check.Closures.FnMutEvidence
