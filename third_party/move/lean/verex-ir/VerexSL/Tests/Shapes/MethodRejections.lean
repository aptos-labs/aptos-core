-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# What a trait-method call must reject

A function generic in a type and in its method (the dictionary of a trait
bound, as a Rust source's generic function over `T : Step` is rendered)
invokes the method twice and states that through `ensures_of` of the method.
A caller passes the method of an implementation and reads its contract
through the callee's.

The positive states what two steps do. The negatives are the three things
the path must refuse: a caller's clause that claims another result than the
method's contract gives through the callee; a caller's clause about the
receiver's state that the contract does not give; and a method whose body
does not do what its own contract states. A caller that holds no contract of
a method it passes on is `VerexSL/Tests/ForwardedMethodEvidence`.
-/

namespace VerexSL.Tests.Shapes.MethodRejections

set_option Elab.async false

verex namespace method_rejections where
  pragma verify = false

  @[method_evidence = 1]
  fun step_twice {T}(
    value : &mut T, __trait_Step_step : Fn(&mut T) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_Step_step, &mut *value)
    invoke(__trait_Step_step, &mut *value)

  spec step_twice where
    pragma opaque
    ensures exists (middle : T),
      ensures_of<__trait_Step_step>(old(value), middle) &&
      ensures_of<__trait_Step_step>(middle, value)

  struct Toggle where
    value : Bool

  trait impl «<Toggle as Step>» where
    fun step(self : &mut Toggle) -> Unit := (*self).value := !(*self).value

  trait impl «<Toggle as Step>» where
    spec step where
      pragma opaque
      ensures self.value == !old(self.value)

  fun twice_ok(value : Bool) -> Bool := do
    let mut state := new Toggle { value }
    let held := &mut state
    step_twice(
      held,
      function[Fn(&mut Toggle) -> Unit has Copy, Drop](«<Toggle as Step>»::step)
    )
    state.value

  spec twice_ok where
    ensures result == value

  -- The result of two steps claimed to be that of one.
  fun twice_result_bad(value : Bool) -> Bool := do
    let mut state := new Toggle { value }
    let held := &mut state
    step_twice(
      held,
      function[Fn(&mut Toggle) -> Unit has Copy, Drop](«<Toggle as Step>»::step)
    )
    state.value

  spec twice_result_bad where
    ensures result == !value

  -- The receiver's state after the call claimed of a receiver the contract
  -- says nothing fixed about: two steps of a method known only by name. The
  -- function holds the method's contract, as a generic function over the
  -- bound is rendered, and hands it to the callee.
  @[method_evidence = 1]
  fun twice_receiver_bad {T}(
    value : &mut T, __trait_Step_step : Fn(&mut T) -> Unit has Copy, Drop
  ) -> Unit := do
    step_twice(value, __trait_Step_step)

  spec twice_receiver_bad where
    ensures ensures_of<__trait_Step_step>(old(value), value)

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `method_rejections.twice_result_bad ["ensures result == !value"]
    (← `(command| verify method_rejections::twice_result_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `method_rejections.twice_receiver_bad
    ["ensures ensures_of<__trait_Step_step>(old(value), value)"]
    (← `(command| verify method_rejections::twice_receiver_bad))

/-- A method that leaves its receiver as it is under a contract that says it
flips it. A method of an implementation is verified where its namespace is
elaborated, so the namespace is a command of its own. -/
def stuckSource : String := "verex namespace stuck_method where
  struct Stuck where
    value : Bool

  trait impl «<Stuck as Step>» where
    fun step(self : &mut Stuck) -> Unit := (*self).value := (*self).value

  trait impl «<Stuck as Step>» where
    spec step where
      pragma opaque
      ensures self.value == !old(self.value)
"

open LoanDeathControls in
run_cmd do
  let .ok command := Lean.Parser.runParserCategory (← Lean.getEnv) `command stuckSource
    | throwError "the namespace of the method that does not flip does not parse"
  let diagnostics ← rejectedDiagnostics `stuck_method.step ⟨command⟩
  -- The namespace is parsed from a string, so its clause is named by its kind.
  let stated := diagnostics.filter fun text =>
    !text.startsWith "verex verification failed: the automatic verification of `"
  let [clause] := stated
    | throwError "a method whose body does not meet its contract: found {diagnostics}"
  unless clause.startsWith "the specification clause `" &&
      clause.endsWith "` is not established" do
    throwError "a method whose body does not meet its contract: found {diagnostics}"

/-! ## Positive controls -/

verify method_rejections::twice_ok

-- The method that flips, with its contract: verified where its namespace is
-- elaborated.
verex namespace flipping_method where
  struct Toggle where
    value : Bool

  trait impl «<Toggle as Step>» where
    fun step(self : &mut Toggle) -> Unit := (*self).value := !(*self).value

  trait impl «<Toggle as Step>» where
    spec step where
      pragma opaque
      ensures self.value == !old(self.value)

end VerexSL.Tests.Shapes.MethodRejections
