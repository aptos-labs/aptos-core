-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

namespace VerexSL.Tests.Check.Closures.StaticMethodEvidence

-- The lowered dictionary interface: a mutable receiver remains explicit.
verex namespace methods where
  @[method_evidence = 1]
  fun twice {T}(value : &mut T, method : Fn(&mut T) -> Unit has Copy, Drop) -> Unit := do
    invoke(method, &mut *value)
    invoke(method, &mut *value)
  spec twice where
    pragma opaque
    ensures exists (middle : T),
      ensures_of<method>(old(value), middle) && ensures_of<method>(middle, value)

  fun bump(value : &mut u64) -> Unit := *value := *value + 1
  spec bump where
    pragma opaque
    ensures value == (old(value) + 1) % 18446744073709551616

  fun toggle(value : &mut Bool) -> Unit := *value := !*value
  spec toggle where
    pragma opaque
    ensures value == !old(value)

  fun count(n : u64) -> u64 := do
    let mut value := n
    twice(&mut value, function[Fn(&mut u64) -> Unit has Copy, Drop](bump))
    value
  spec count where
    ensures result == (n + 2) % 18446744073709551616

  fun flip(b : Bool) -> Bool := do
    let mut value := b
    twice(&mut value, function[Fn(&mut Bool) -> Unit has Copy, Drop](toggle))
    value
  spec flip where
    ensures result == b

-- Both callers must reuse the same checked generic theorem, not re-prove a
-- specialization of the body. The public interface must retain its drop bound.
open Lean Elab Command in
run_cmd do
  let base := `VerexSL.Tests.Check.Closures.StaticMethodEvidence.methods
  let environment ← getEnv
  let some theorem_ := environment.find? (base ++ `twice.verified)
    | throwError "missing public generic theorem"
  unless theorem_.type.getUsedConstants.contains
      ``VerexIR.Proofs.Denote.DroppedTypeArgumentsTrivial do
    throwError "the public Rust theorem lost its trivial-drop requirement"
  for caller in ["count", "flip"] do
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
    unless used.contains (base ++ `twice.typedSemanticsVerified) do
      throwError "{caller} did not reuse the generic method theorem"

end VerexSL.Tests.Check.Closures.StaticMethodEvidence
