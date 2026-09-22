-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

namespace LeanerLang.Tests.VerificationReferences

-- Calls consume the callee's proved contract through the shared normalizer.
set_option leaner.route "native"

leaner module 0x42::verification_references where
  struct Pair has Key where
    left : u64
    right : u64

  public fun reborrow(slot : &mut u64) -> &mut u64 := &mut *slot

  spec reborrow where
    ensures result == old(slot) && slot == result
    aborts_if false

  public fun forward_reborrow(slot : &mut u64) -> &mut u64 :=
    core.call reborrow::<>(&mut *slot)

  spec forward_reborrow where
    ensures result == old(slot) && slot == result
    aborts_if false

  public fun set_through_call(slot : &mut u64) -> Unit := do
    let returned := core.call reborrow::<>(&mut *slot)
    *returned := 7

  spec set_through_call where
    ensures slot == 7
    aborts_if false

  public fun set_through_forward(slot : &mut u64) -> Unit := do
    let returned := core.call forward_reborrow::<>(&mut *slot)
    *returned := 9

  spec set_through_forward where
    ensures slot == 9
    aborts_if false

  public fun set_then_read(slot : &mut u64) -> u64 := do
    let returned := core.call reborrow::<>(&mut *slot)
    *returned := 7
    return *slot

  spec set_then_read where
    ensures result == 7 && slot == 7
    aborts_if false

  public fun choose_reborrow(take_left : Bool, left : &mut u64,
      right : &mut u64) -> &mut u64 :=
    if take_left then &mut *left else &mut *right

  spec choose_reborrow where
    ensures ((take_left && result == old(left)) ||
      (!take_left && result == old(right))) &&
      left == old(left) && right == old(right)
    aborts_if false

  public fun return_pair(left : &mut u64, right : &mut u64) ->
      (&mut u64, &mut u64) :=
    (&mut *left, &mut *right)

  spec return_pair where
    ensures spec.result[0] == old(left) && spec.result[1] == old(right) &&
      left == spec.result[0] && right == spec.result[1]
    aborts_if false

  public fun set_through_pair(left : &mut u64, right : &mut u64) -> Unit := do
    let (returned_left, returned_right) :=
      core.call return_pair::<>(&mut *left, &mut *right)
    *returned_left := 11
    *returned_right := 13

  spec set_through_pair where
    ensures left == 11 && right == 13
    aborts_if false

  public fun project_left(pair : &mut Pair) -> &mut u64 :=
    &mut pair.left

  spec project_left where
    ensures result == old(pair).left && pair.left == result &&
      pair.right == old(pair).right
    aborts_if false

  public fun set_projected(pair : &mut Pair) -> Unit := do
    let returned := core.call project_left::<>(&mut *pair)
    *returned := 17

  spec set_projected where
    ensures pair.left == 17 && pair.right == old(pair).right
    aborts_if false

  -- The VM allows returning a parameter-derived reference, not borrowing
  -- storage in the callee and returning that global-rooted reference.
  public fun global_left(pair : &mut Pair) -> &mut u64 :=
    &mut pair.left

  spec global_left where
    ensures result == old(pair).left && pair.left == result &&
      pair.right == old(pair).right
    aborts_if false

  public fun set_global_left(address : Address) -> Unit := do
    let resource := &mut Pair[address]
    let returned := core.call global_left::<>(&mut *resource)
    *returned := 19

  spec set_global_left where
    requires exists<Pair>(address)
    ensures global<Pair>(address).left == 19 &&
      global<Pair>(address).right == old(global<Pair>(address)).right
    aborts_if false
    modifies global<Pair>(address)

  verify reborrow
  verify forward_reborrow
  verify set_through_call
  verify set_through_forward
  verify set_then_read
  verify choose_reborrow
  verify return_pair
  verify set_through_pair
  verify project_left
  verify set_projected
  verify global_left
  verify set_global_left


/-! Nonescaping returned-reference uses have an analysis-authored death;
references actually carried by a function result deliberately remain live
for the call boundary to transfer. These kernel-reduced checks avoid turning
the generated unit into executable test code. -/
example :
    «0x42».verification_references.unit.borrowCertificates[0]!
      |>.loans[0]!.deaths.isEmpty = true := by decide
example :
    «0x42».verification_references.unit.borrowCertificates[1]!
      |>.loans[0]!.deaths.isEmpty = true := by decide
example :
    «0x42».verification_references.unit.borrowCertificates[2]!
      |>.loans[0]!.deaths.isEmpty = false := by decide
example :
    «0x42».verification_references.unit.borrowCertificates[3]!
      |>.loans[0]!.deaths.isEmpty = false := by decide
example :
    «0x42».verification_references.unit.borrowCertificates[4]!
      |>.loans[0]!.deaths.isEmpty = false := by decide
example :
    «0x42».verification_references.unit.borrowCertificates[11]!
      |>.loans[0]!.deaths.isEmpty = false := by decide
example :
    «0x42».verification_references.unit.borrowDiagnostics.isEmpty = true := by
  decide

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let initial ← singleResourceState `«0x42».verification_references "Pair" "0x2"
    #[.integer 7, .integer 11]
  let final ← singleResourceState `«0x42».verification_references "Pair" "0x2"
    #[.integer 19, .integer 11] 3
  assertRunsState `«0x42».verification_references #[
    ⟨"set_global_left", #[.address "0x2"], .returned #[], initial, final⟩]

end LeanerLang.Tests.VerificationReferences

/-! The returned reference fixes the source parameter's value at the call
boundary.  In particular, contract generation must not satisfy an arbitrary
post-state claim by choosing an unrelated existential exit value. -/

namespace LeanerLang.Tests.VerificationReferencesNegative

leaner module 0x43::verification_references_negative where
  public fun reborrow_bad(slot : &mut u64) -> &mut u64 := &mut *slot

  spec reborrow_bad where
    ensures result == old(slot) && slot == result + 1
    aborts_if false

/-! The false post-state relation must be rejected. -/

/--
error: the specification clause `ensures result == old(slot) && slot == result + 1` is not established
---
error: leaner verification failed
-/
#guard_msgs in
  #leaner_verify 0x43::verification_references_negative::reborrow_bad

end LeanerLang.Tests.VerificationReferencesNegative
