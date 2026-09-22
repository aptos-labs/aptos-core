-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"

leaner module 0x42::native_arithmetic where
  fun add(left : u64, right : u64) -> u64 := left + right
  spec add where
    ensures result == left + right
    aborts_if left + right > 18446744073709551615
  verify add

  fun subtract(left : u8, right : u8) -> u8 := left - right
  spec subtract where
    ensures result == left - right
    aborts_if left < right
  verify subtract

  fun multiply(left : u8, right : u8) -> u8 := left * right
  spec multiply where
    ensures result == left * right
    aborts_if left * right > 255
  verify multiply

  fun reverse(left : u8, right : u8) -> u8 := right - left
  spec reverse where
    ensures result == right - left
    aborts_if right < left
  verify reverse

  fun repeated(value : u8) -> u8 := value + value
  spec repeated where
    ensures result == value + value
    aborts_if value + value > 255
  verify repeated

  fun bounded(left : u8, right : u8) -> u8 := left + right
  spec bounded where
    requires left + right <= 255
    ensures result == left + right
    aborts_if false
  verify bounded

  fun successor(value : u64) -> u64 := value + 1
  spec successor where
    ensures result == value + 1
    aborts_if value == 18446744073709551615
  verify successor

  fun complement(value : u8) -> u8 := 255 - value
  spec complement where
    ensures result == 255 - value
    aborts_if false
  verify complement

  fun double(value : u8) -> u8 := 2 * value
  spec double where
    ensures result == 2 * value
    aborts_if value > 127
  verify double

  fun no_overflow(left : u8, right : u8) -> u8 := left + right
  spec no_overflow where
    ensures result == left + right
    aborts_if false

  fun spurious_abort(left : u8, right : u8) -> u8 := left + right
  spec spurious_abort where
    ensures result == left + right
    aborts_if left + right > 255
    aborts_if left == 0

  fun wrong_result(left : u8, right : u8) -> u8 := left + right
  spec wrong_result where
    ensures result == left
    aborts_if left + right > 255

#leaner_prepare 0x42::native_arithmetic::no_overflow
#leaner_prepare 0x42::native_arithmetic::spurious_abort
#leaner_prepare 0x42::native_arithmetic::wrong_result

/--
error: the specification clause `aborts_if false` is not established
---
error: native computation generation failed for `no_overflow`
-/
#guard_msgs in
#leaner_verify 0x42::native_arithmetic::no_overflow

-- Sufficiency must be checked too: a normally returning computation cannot
-- claim that zero on the left always aborts. The rejection's context is not
-- a baseline; absence of every generated artifact below asserts rejection.
#guard_msgs (drop error) in
#leaner_verify 0x42::native_arithmetic::spurious_abort

/--
error: the specification clause `ensures result == left` is not established
---
error: native computation generation failed for `wrong_result`
-/
#guard_msgs in
#leaner_verify 0x42::native_arithmetic::wrong_result

open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_arithmetic
  for function in ["no_overflow", "spurious_abort", "wrong_result"] do
    for suffix in [`computation, `nativeSummary, `computationState,
        `computationVerified, `computationRepresents, `typedVerified] do
      let name := Name.str artifactRoot function ++ suffix
      if (← getEnv).contains name then throwError "rejected arithmetic leaked {name}"
    let input ← LeanerLang.Contract.prepareVerification Syntax.missing
      #["0x42", "native_arithmetic"] function
    if (LeanerLang.NativeRegistry.entries.getState (← getEnv)).contains input.generated.relation then
      throwError "rejected arithmetic leaked its reusable registry entry"
  for function in ["add", "subtract", "multiply", "reverse", "repeated", "bounded",
      "successor", "complement", "double"] do
    for suffix in [`computation, `nativeSummary, `computationVerified] do
      let root := Name.str artifactRoot function ++ suffix
      let nativeValues := suffix == `computation
      let mut pending := #[root]
      let mut visited : Array Name := #[]
      while let some name := pending.back? do
        pending := pending.pop
        if visited.contains name then continue
        visited := visited.push name
        let some declaration := (← getEnv).find? name
          | throwError "missing native arithmetic artifact {name}"
        let constants := declaration.type.getUsedConstants ++
          ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
        for dependency in constants do
          if dependency == ``LeanerIR.RuntimeFrame ||
              dependency == ``LeanerIR.Proofs.typedFunction ||
              dependency == ``LeanerIR.Proofs.decodeSpec ||
              (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
              dependency.getString! == "computationRepresents" ||
              dependency.getString! == "computationState" ||
              dependency == ``sorryAx ||
              (nativeValues && dependency == ``LeanerIR.RuntimeValue) then
            throwError "native arithmetic {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency
