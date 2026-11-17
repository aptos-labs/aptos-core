-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

set_option Elab.async false

/-! ## Method evidence of a function whose body is not verified

`pragma verify = false` takes a function's body out of the automatic
verification. What a run of a function value the proof cannot see needs is
owed by the function's interface: a parameter of function type, a condition
stating `aborts_of` or `result_of`, a behavioral data invariant of a value it
takes or returns. Its contract therefore states that evidence to its callers,
and an explicit `verify` item for it receives the same. The body stays out:
its inner specifications are not read and its callees are not followed, so a
function whose interface owes nothing assumes nothing.

Negatives first: a caller that holds no contract of the function value it
hands to such a callee does not obtain the callee's behavior. -/

open Lean Elab Command VerexIR VerexSL

namespace VerexSL.Tests.UnverifiedMethodEvidence

-- ============================================================================
-- 1. Negative control
-- ============================================================================

/-- The module of the negative control: `unit_mut`, whose body is not
verified, assumes the contract of its function-typed parameter; `forward`
passes `call` on inside the adapter and assumes no contract of it. -/
def uncheckedSource : String := "verex namespace unchecked where
  @[method_evidence = 1]
  fun unit_mut {F}(
    callback : &mut F, call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call_mut, &mut *callback)

  spec unit_mut where
    pragma opaque
    pragma verify = false
    ensures ensures_of<call_mut>(old(callback), callback)

  @[method_evidence = 1, shared_method_adapter = 1]
  fun shared_as_mut {F}(
    receiver : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call, &*receiver)

  spec shared_as_mut where
    pragma opaque
    ensures receiver == old(receiver)
    ensures ensures_of<call>(old(receiver))

  fun forward {F}(
    callback : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](shared_as_mut::<F>, _, call)
    )

  spec forward where
    pragma opaque
    ensures ensures_of<call>(old(callback))
"

open LoanDeathControls in
run_cmd do
  let .ok command := Parser.runParserCategory (← getEnv) `command uncheckedSource
    | throwError "the negative control does not parse"
  let diagnostics ← rejectedDiagnostics `unchecked.forward ⟨command⟩
  -- The module is parsed from a string, so a clause is named by its kind: the
  -- one clause of `forward`, and the adapter's premise at the call, which the
  -- unverified callee's contract owes.
  let stated := diagnostics.filter fun text =>
    !text.startsWith "verex verification failed: the automatic verification of `forward` failed;"
  let [premise, clause] := stated
    | throwError "a caller without the dictionary's contract: found {diagnostics}"
  unless premise == "the precondition of `shared_as_mut` is not established" &&
      clause.startsWith "the specification clause `" && clause.endsWith "` is not established" do
    throwError "a caller without the dictionary's contract: found {diagnostics}"

-- ============================================================================
-- 2. An explicit `verify` item after `pragma verify = false`
-- ============================================================================

verex namespace explicitly_verified where
  @[method_evidence = 1]
  fun unit_mut {F}(
    callback : &mut F, call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call_mut, &mut *callback)

  spec unit_mut where
    pragma opaque
    ensures ensures_of<call_mut>(old(callback), callback)

  @[method_evidence = 1, shared_method_adapter = 1]
  fun shared_as_mut {F}(
    receiver : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call, &*receiver)

  spec shared_as_mut where
    pragma opaque
    ensures receiver == old(receiver)
    ensures ensures_of<call>(old(receiver))

  -- The body is out of the automatic verification, and the item asks for it.
  @[method_evidence = 1]
  fun forward {F}(
    callback : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](shared_as_mut::<F>, _, call)
    )

  spec forward where
    pragma opaque
    pragma verify = false
    ensures callback == old(callback)
    ensures ensures_of<call>(old(callback))

  verify forward

run_cmd do
  let env ← getEnv
  let base := `VerexSL.Tests.UnverifiedMethodEvidence ++ `explicitly_verified.forward
  let some verified := env.find? (base ++ `verified)
    | throwError "the explicitly verified function did not verify"
  let constants := verified.type.getUsedConstants
  for assumption in [``VerexIR.NativesTyped, ``VerexIR.NativesShift,
      ``VerexIR.Proofs.RunsAgree, ``VerexIR.Proofs.OwnersRepresentable] do
    unless constants.contains assumption do
      throwError s!"the theorem of a function verified by an item does not assume {assumption}"
  -- Its contract assumes the contract of its function-typed parameter, which
  -- only the `assumes` field states, and not `True` in its place.
  let some contract := env.find? `explicitly_verified.forward.typedContract
    | throwError "the explicitly verified function has no contract"
  unless contract.value!.getUsedConstants.contains ``VerexIR.Proofs.MethodContract do
    throwError "the contract of a function verified by an item assumes no method evidence"

-- ============================================================================
-- 3. A caller of a function whose body is not verified
-- ============================================================================

verex namespace trusted_callee where
  @[method_evidence = 1]
  fun unit_mut {F}(
    callback : &mut F, call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call_mut, &mut *callback)

  spec unit_mut where
    pragma opaque
    pragma verify = false
    ensures ensures_of<call_mut>(old(callback), callback)

  @[method_evidence = 1, shared_method_adapter = 1]
  fun shared_as_mut {F}(
    receiver : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call, &*receiver)

  spec shared_as_mut where
    pragma opaque
    ensures receiver == old(receiver)
    ensures ensures_of<call>(old(receiver))

  -- The caller holds the contract of `call` and establishes what the
  -- unverified callee's contract assumes.
  @[method_evidence = 1]
  fun forward {F}(
    callback : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](shared_as_mut::<F>, _, call)
    )

  spec forward where
    pragma opaque
    ensures callback == old(callback)
    ensures ensures_of<call>(old(callback))

run_cmd do
  let env ← getEnv
  unless env.contains
      (`VerexSL.Tests.UnverifiedMethodEvidence ++ `trusted_callee.forward.verified) do
    throwError "the caller of the unverified callee did not verify"
  let some unit := VerexSL.registeredUnit? env `trusted_callee
    | throwError "the module of the unverified callee was not registered"
  let some (namespaceIndex, ns, _, declaration) := Contract.findFunction? unit "unit_mut"
    | throwError "the unverified callee was not found"
  unless Contract.assumesTyping unit ⟨namespaceIndex⟩ ns declaration do
    throwError "the unverified callee's interface owes no method evidence"

-- ============================================================================
-- 4. What an interface owing nothing assumes, and what a body keeps to itself
-- ============================================================================

verex module 0x4d::gated where
  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > 18446744073709551615
    ensures result == x + y

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  -- No parameter of function type and no behavioral clause.
  fun plain(x : u64) -> u64 := x
  spec plain where
    pragma verify = false
    aborts_if false
    ensures result == x

  fun use_plain(x : u64) -> u64 := plain(x)
  spec use_plain where
    aborts_if false
    ensures result == x

  -- A body the automatic verification does not walk: its call to a
  -- higher-order function and its inner `aborts_of` are not read.
  fun hidden(x : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, 1, _)
    spec assume !aborts_of<f>(x)
    apply(f, x)
  spec hidden where
    pragma verify = false
    aborts_if false
    ensures result == x + 1

run_cmd do
  let env ← getEnv
  let some unit := VerexSL.registeredUnit? env `«0x4d».gated
    | throwError "the gated module was not registered"
  let assumes (key : String) : CommandElabM Bool := do
    let some (namespaceIndex, ns, _, declaration) := Contract.findFunction? unit key
      | throwError s!"`{key}` was not found"
    return Contract.assumesTyping unit ⟨namespaceIndex⟩ ns declaration
  if ← assumes "plain" then
    throwError "a function owing no method evidence assumes typing"
  if ← assumes "hidden" then
    throwError "the specifications and callees of a body that is not walked were read"
  unless ← assumes "apply" do
    throwError "a higher-order function assumes no typing"
  -- The caller's own interface owes nothing either, so its theorem assumes
  -- neither the typing nor the natives that preserve it.
  let some verified := env.find?
      (`VerexSL.Tests.UnverifiedMethodEvidence ++ `«0x4d».gated.use_plain.verified)
    | throwError "the caller of the unverified function did not verify"
  if verified.type.getUsedConstants.contains ``VerexIR.NativesTyped then
    throwError "the caller of a function owing no method evidence assumes typing"

-- ============================================================================
-- 5. The results of a function value whose target is not verified
-- ============================================================================

/-! A higher-order function whose parameter returns a type that states a data
invariant returns a valid result only where the invocations of the value do,
so its contract owes its callers that fact, with a body that is verified or
not. A call that passes a named function pays it from that function's
theorem. A function no verification has checked has no theorem, and the call
is refused: of `two_hundred`, whose result breaks the invariant, through a
callee whose contract says nothing of the value's behavior, so that no step
of the caller's proof would walk the function; and of `fifty`, whose result
keeps the invariant, until `fifty` is verified.

A caller's theorem takes the contract of such a callee as a `Satisfies`
hypothesis. A contract promising valid results of every function value would
therefore make the theorem of `bounded` conditional on that hypothesis, not
false; that no body `invoke(f, x)` satisfies such a contract at `two_hundred`
is not proved here. -/

verex module 0x4e::unverified_target where
  pragma verify = false

  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  fun pass(f : Fn(u64) -> Percent has Copy, Drop, x : u64) -> Percent := invoke(f, x)
  spec pass where
    pragma opaque = true
    pragma verify = false
    aborts_if false

  fun run(f : Fn(u64) -> Percent has Copy, Drop, x : u64) -> Percent := invoke(f, x)
  spec run where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures ensures_of<f>(x, result)

  fun two_hundred(x : u64) -> Percent := new Percent { value := 200 }
  spec two_hundred where
    aborts_if false

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  -- The claim is what the invariant of the result type would give.
  fun bounded(x : u64) -> Percent :=
    pass(function[Fn(u64) -> Percent has Copy, Drop](two_hundred), x)
  spec bounded where
    aborts_if false
    ensures result.value <= 100

  fun through_fifty(x : u64) -> Percent :=
    run(function[Fn(u64) -> Percent has Copy, Drop](fifty), x)
  spec through_fifty where
    aborts_if false
    ensures result.value == 50

open LoanDeathControls in
run_cmd do
  let refused (base : Name) (command : TSyntax `command) : CommandElabM (List String) := do
    return (← rejectedDiagnostics base command).filter
      (!·.startsWith "verex verification failed: ")
  let invalid ← refused `«0x4e».unverified_target.bounded
    (← `(command| verify 0x4e::unverified_target::bounded))
  unless invalid ==
      ["the data invariant `invariant value <= 100` does not hold where a value is constructed",
        "the precondition of `pass` at call 1 is not established"] do
    throwError "a function value whose target returns an invalid result: found {invalid}"
  let unverified ← refused `«0x4e».unverified_target.through_fifty
    (← `(command| verify 0x4e::unverified_target::through_fifty))
  unless unverified == ["the precondition of `run` at call 1 is not established"] do
    throwError "a function value whose target is not verified: found {unverified}"

-- The target's theorem is the evidence: with it the same call verifies.
verify 0x4e::unverified_target::fifty
verify 0x4e::unverified_target::through_fifty

-- ============================================================================
-- 6. The frame of a function value that takes a reference
-- ============================================================================

/-! A `modifies_of` clause binds its formals to an invocation's arguments, and
a formal of reference type is read as a parameter of that type is: a mutable
reference at the value it holds where the invocation starts, a shared
reference at what it refers to. The contract of a function under `pragma
verify = false` is built where a caller uses it, so each frame below is
stated by its caller's verification.

The executable frame (`FramedAt`) is stated of a parameter row and a result
row that hold no reference and is vacuous otherwise. At a row with a mutable
reference a caller's check of the function value it passes therefore says
nothing of that value's frame, and a proof through an unverified opaque
callee assumes that callee's whole contract. So the cells of `apply` and
`apply_at` pin only that the frame is stated and the call goes through the
contract: the positive verifies and the negative fails on its own clause
alone. A row with a shared reference states a frame a caller must establish:
a function that writes outside it is refused at the call.

In a module verified through its model the frame of a row with a mutable
reference is stated over the model's invocations (`FramedAtOf`) and is
established where the value is passed or stored. The last module pins, for a
function-typed parameter and for a function-valued field, that a target is
read at the value the reference holds where the invocation starts. -/

verex module 0x4e::reference_frames where
  pragma verify = false

  struct Counter has Key where
    total : u64

  struct Config has Key where
    total : u64

  -- A formal of mutable reference type that no target reads.
  fun apply(f : Fn(&mut u64) -> Unit has Drop, x : &mut u64) -> Unit := invoke(f, x)
  spec apply where
    pragma opaque = true
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(old(x), x)
    modifies *
    modifies_of<f>(v : &mut u64) global<Counter>(@0x42)

  fun debit(v : &mut u64) -> Unit := do
    let total := &mut Counter[@0x42].total
    *total := *total - *v
    *v := 0
  spec debit where
    aborts_if !exists<Counter>(@0x42)
    aborts_if global<Counter>(@0x42).total < v
    ensures v == 0
    ensures global<Counter>(@0x42).total == old(global<Counter>(@0x42).total) - old(v)
    modifies global<Counter>(@0x42)

  fun settle(x : &mut u64) -> Unit :=
    apply(function[Fn(&mut u64) -> Unit has Drop](debit), x)
  spec settle where
    aborts_if !exists<Counter>(@0x42)
    aborts_if global<Counter>(@0x42).total < x
    ensures x == 0
    ensures global<Counter>(@0x42).total == old(global<Counter>(@0x42).total) - old(x)
    modifies *

  fun settle_bad(x : &mut u64) -> Unit :=
    apply(function[Fn(&mut u64) -> Unit has Drop](debit), x)
  spec settle_bad where
    aborts_if !exists<Counter>(@0x42)
    aborts_if global<Counter>(@0x42).total < x
    ensures x == 1
    modifies *

  -- A formal of mutable reference type that a target reads.
  fun apply_at(f : Fn(&mut Address) -> u64 has Drop, a : &mut Address) -> u64 := invoke(f, a)
  spec apply_at where
    pragma opaque = true
    pragma verify = false
    pragma aborts_if_is_partial
    modifies *
    modifies_of<f>(owner : &mut Address) global<Counter>(*owner)

  fun reset_at(owner : &mut Address) -> u64 := do
    let total := &mut Counter[*owner].total
    *total := 0
    0
  spec reset_at where
    pragma aborts_if_is_partial
    modifies global<Counter>(owner)

  fun reset(a : &mut Address) -> u64 :=
    apply_at(function[Fn(&mut Address) -> u64 has Drop](reset_at), a)
  spec reset where
    pragma aborts_if_is_partial
    modifies *

  -- A formal of shared reference type that a target reads.
  fun apply_shared(f : Fn(&Address) -> u64 has Drop, a : &Address) -> u64 := invoke(f, a)
  spec apply_shared where
    pragma opaque = true
    pragma verify = false
    pragma aborts_if_is_partial
    modifies *
    modifies_of<f>(owner : &Address) global<Counter>(*owner)

  fun clear_counter(owner : &Address) -> u64 := do
    let total := &mut Counter[*owner].total
    *total := 0
    0
  spec clear_counter where
    pragma aborts_if_is_partial
    modifies global<Counter>(owner)

  fun clear_config(owner : &Address) -> u64 := do
    let total := &mut Config[*owner].total
    *total := 0
    0
  spec clear_config where
    pragma aborts_if_is_partial
    modifies global<Config>(owner)

  fun clears_counter(a : &Address) -> u64 :=
    apply_shared(function[Fn(&Address) -> u64 has Drop](clear_counter), a)
  spec clears_counter where
    pragma aborts_if_is_partial
    modifies *

  fun clears_config(a : &Address) -> u64 :=
    apply_shared(function[Fn(&Address) -> u64 has Drop](clear_config), a)
  spec clears_config where
    pragma aborts_if_is_partial
    modifies *

open LoanDeathControls in
run_cmd do
  let refused (base : Name) (command : TSyntax `command) : CommandElabM (List String) := do
    return (← rejectedDiagnostics base command).filter
      (!·.startsWith "verex verification failed: ")
  let claimed ← refused `«0x4e».reference_frames.settle_bad
    (← `(command| verify 0x4e::reference_frames::settle_bad))
  unless claimed == ["the specification clause `ensures x == 1` is not established"] do
    throwError "a false clause after a call that takes a mutable reference: found {claimed}"
  let outside ← refused `«0x4e».reference_frames.clears_config
    (← `(command| verify 0x4e::reference_frames::clears_config))
  unless outside == ["the precondition of `apply_shared` at call 1 is not established"] do
    throwError "a function value that writes outside its frame: found {outside}"

verify 0x4e::reference_frames::settle
verify 0x4e::reference_frames::reset
verify 0x4e::reference_frames::clears_counter

verex module 0x4e::model_reference_frames where
  pragma verify = false

  struct «Ghost$calls» has Copy, Drop, Store, Key where
    v : u64

  spec «Ghost$calls» where
    pragma ghost_memory

  struct Counter has Key where
    total : u64

  struct Config has Key where
    total : u64

  fun apply_at(f : Fn(&mut Address) -> u64 has Drop, a : &mut Address) -> u64 := invoke(f, a)
  spec apply_at where
    pragma opaque = true
    pragma verify = false
    pragma aborts_if_is_partial
    modifies *
    modifies_of<f>(owner : &mut Address) global<Counter>(*owner)

  -- The same frame of a function-valued field.
  struct Stored has Copy, Drop, Store where
    action : Fn(&mut Address) -> u64 has Copy, Drop, Store
  spec Stored where
    modifies_of<action>(owner : &mut Address) global<Counter>(*owner)

  public fun reset_at(owner : &mut Address) -> u64 := do
    let total := &mut Counter[*owner].total
    *total := 0
    0
  spec reset_at where
    pragma aborts_if_is_partial
    modifies global<Counter>(owner)

  public fun reset_config(owner : &mut Address) -> u64 := do
    let total := &mut Config[*owner].total
    *total := 0
    0
  spec reset_config where
    pragma aborts_if_is_partial
    modifies global<Config>(owner)

  -- Writes the counter at the address the reference holds after the write.
  public fun reset_moved(owner : &mut Address) -> u64 := do
    *owner := @0x43
    let total := &mut Counter[*owner].total
    *total := 0
    0
  spec reset_moved where
    pragma aborts_if_is_partial
    modifies global<Counter>(@0x43)

  fun reset(a : &mut Address) -> u64 :=
    apply_at(function[Fn(&mut Address) -> u64 has Drop](reset_at), a)
  spec reset where
    pragma aborts_if_is_partial
    modifies *

  fun reset_elsewhere(a : &mut Address) -> u64 :=
    apply_at(function[Fn(&mut Address) -> u64 has Drop](reset_moved), a)
  spec reset_elsewhere where
    pragma aborts_if_is_partial
    modifies *

  fun store() -> Stored := new Stored {
    action := function[Fn(&mut Address) -> u64 has Copy, Drop, Store](reset_at) }
  spec store where
    aborts_if false

  fun store_moved() -> Stored := new Stored {
    action := function[Fn(&mut Address) -> u64 has Copy, Drop, Store](reset_moved) }
  spec store_moved where
    aborts_if false

  fun store_config() -> Stored := new Stored {
    action := function[Fn(&mut Address) -> u64 has Copy, Drop, Store](reset_config) }
  spec store_config where
    aborts_if false

open LoanDeathControls in
run_cmd do
  let refused (base : Name) (command : TSyntax `command) : CommandElabM (List String) := do
    return (← rejectedDiagnostics base command).filter
      (!·.startsWith "verex verification failed: ")
  let moved ← refused `«0x4e».model_reference_frames.reset_elsewhere
    (← `(command| verify 0x4e::model_reference_frames::reset_elsewhere))
  unless moved == ["the precondition of `apply_at` at call 1 is not established"] do
    throwError "a write at the address a mutable reference holds after a write: found {moved}"
  -- A field's frame is the field's own fact, owed where the value is made.
  let field := "the data invariant `action : Fn(&mut Address) -> u64 has Copy, Drop, Store` \
    does not hold where a value is constructed"
  let storedMoved ← refused `«0x4e».model_reference_frames.store_moved
    (← `(command| verify 0x4e::model_reference_frames::store_moved))
  unless storedMoved == [field] do
    throwError "a stored function writing at the address its reference holds after a write: \
      found {storedMoved}"
  let storedOutside ← refused `«0x4e».model_reference_frames.store_config
    (← `(command| verify 0x4e::model_reference_frames::store_config))
  unless storedOutside == [field] do
    throwError "a stored function writing outside its field's frame: found {storedOutside}"

verify 0x4e::model_reference_frames::reset
verify 0x4e::model_reference_frames::store

end VerexSL.Tests.UnverifiedMethodEvidence
