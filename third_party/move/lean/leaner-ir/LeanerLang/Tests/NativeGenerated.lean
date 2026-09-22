-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

-- Agreement lookup certificates must not compute the complete arena length
-- before visiting a shallow slot, even when the payload remains symbolic.
set_option maxHeartbeats 1000 in
example {α : Type} (value : α) :
    (List.replicate 8192 value).toArray[7]? = some value := by
  leaner_arena_rfl

set_option Elab.async false
set_option leaner.route "native"

leaner module 0x42::native_generated where
  fun forward {T : type}(item : T) -> T := item
  spec forward where
    ensures result == item
    aborts_if false
  verify forward

  fun forward_again {T : type}(item : T) -> T := core.call forward::<T>(item)
  spec forward_again where
    ensures result == item
    aborts_if false
  verify forward_again

  fun integer(item : u64) -> u64 := core.call forward_again::<u64>(item)
  spec integer where
    ensures result == item
    aborts_if false
  verify integer

  fun boolean(item : Bool) -> Bool := core.call forward::<Bool>(item)
  spec boolean where
    ensures result == item
    aborts_if false
  verify boolean

  fun restricted(item : u64) -> u64 := item
  spec restricted where
    requires item > 0
    ensures result == item
    aborts_if false
  verify restricted

  fun permitted(item : u64) -> u64 := core.call restricted::<>(item)
  spec permitted where
    requires item > 1
    ensures result == item
    aborts_if false
  verify permitted

  fun unpermitted(item : u64) -> u64 := core.call restricted::<>(item)
  spec unpermitted where
    ensures result == item
    aborts_if false

  fun wrong(item : u64) -> u64 := item
  spec wrong where
    ensures result > item
    aborts_if false

  fun wrong_caller(item : u64) -> u64 := core.call forward::<u64>(item)
  spec wrong_caller where
    ensures result > item
    aborts_if false

#leaner_prepare 0x42::native_generated::unpermitted
#leaner_prepare 0x42::native_generated::wrong
#leaner_prepare 0x42::native_generated::wrong_caller

/--
error: certified closing: unsupported obligation shape 0 < args.item.val
---
error: native computation generation failed for `unpermitted`
-/
#guard_msgs in
#leaner_verify 0x42::native_generated::unpermitted

/--
error: the specification clause `ensures result > item` is not established
---
error: native computation generation failed for `wrong`
-/
#guard_msgs in
#leaner_verify 0x42::native_generated::wrong

/--
error: the specification clause `ensures result > item` is not established
---
error: native computation generation failed for `wrong_caller`
-/
#guard_msgs in
#leaner_verify 0x42::native_generated::wrong_caller

-- A failed command must roll back generated declarations, not leave a
-- partial/admitted certificate available to the next verification.
open Lean Elab Command in
run_cmd do
  for function in ["unpermitted", "wrong", "wrong_caller"] do
    for suffix in [`computation, `pureVerified, `computationVerified, `computationRepresents] do
      let name := Name.str `«0x42».native_generated function ++ suffix
      if (← getEnv).contains name then throwError "failed generation leaked {name}"

-- Audit each generated proof's local dependency closure. Callee result
-- facts may be consumed, but the execution certificate and the stronger
-- semantic purity certificate must not be used to discharge caller VCs.
open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_generated
  for function in ["forward", "forward_again", "integer", "boolean", "restricted", "permitted"] do
    for suffix in [`computation, `pureVerified, `computationVerified] do
      let root := Name.str artifactRoot function ++ suffix
      let nativeValues := suffix == `computation
      let mut pending := #[root]
      let mut visited : Array Name := #[]
      while let some name := pending.back? do
        pending := pending.pop
        if visited.contains name then continue
        visited := visited.push name
        let some declaration := (← getEnv).find? name
          | throwError "missing native artifact {name}"
        let constants := declaration.type.getUsedConstants ++
          ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
        for dependency in constants do
          if dependency == ``LeanerIR.RuntimeFrame ||
              dependency == ``LeanerIR.Proofs.typedFunction ||
              dependency == ``LeanerIR.Proofs.decodeSpec ||
              (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
              dependency.getString! == "computationRepresents" ||
              dependency.getString! == "computationPure" ||
              dependency == ``sorryAx ||
              (nativeValues && dependency == ``LeanerIR.RuntimeValue) then
            throwError "native artifact {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency
