-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"

#leaner_measure

leaner module 0x42::native_calls where
  fun increment(value : u8) -> u8 := value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value == 255
  verify increment

  fun call_increment(value : u8) -> u8 := core.call increment::<>(value)
  spec call_increment where
    ensures result == value + 1
    aborts_if value == 255
  verify call_increment

  fun call_again(value : u8) -> u8 := core.call call_increment::<>(value)
  spec call_again where
    ensures result == value + 1
    aborts_if value == 255
  verify call_again

  fun bounded(value : u8) -> u8 := core.call increment::<>(value)
  spec bounded where
    requires value < 250
    ensures result == value + 1
    aborts_if false
  verify bounded

  fun permitted(value : u8) -> u8 := core.call bounded::<>(value)
  spec permitted where
    requires value < 100
    ensures result == value + 1
    aborts_if false
  verify permitted

  fun add(left : u8, right : u8) -> u8 := left + right
  spec add where
    ensures result == left + right
    aborts_if left + right > 255
  verify add

  fun reversed(left : u8, right : u8) -> u8 := core.call add::<>(right, left)
  spec reversed where
    ensures result == right + left
    aborts_if right + left > 255
  verify reversed

  fun unreported(value : u8) -> u8 := value + 1
  spec unreported where
    ensures result == value + 1
  verify unreported

  fun weak_caller(value : u8) -> u8 := core.call unreported::<>(value)
  spec weak_caller where
    ensures result == value + 1
  verify weak_caller

  fun omitted_abort(value : u8) -> u8 := core.call increment::<>(value)
  spec omitted_abort where
    ensures result == value + 1
    aborts_if false

  fun spurious_abort(value : u8) -> u8 := core.call increment::<>(value)
  spec spurious_abort where
    ensures result == value + 1
    aborts_if value == 255 || value == 0

  fun missing_precondition(value : u8) -> u8 := core.call bounded::<>(value)
  spec missing_precondition where
    ensures result == value + 1
    aborts_if false

  fun wrong_result(value : u8) -> u8 := core.call increment::<>(value)
  spec wrong_result where
    ensures result == value + 2
    aborts_if value == 255

  -- True of the implementation, but not promised by this callee's contract.
  -- A modular caller must reject it, not rediscover the arithmetic body.
  fun hidden_abort(value : u8) -> u8 := core.call unreported::<>(value)
  spec hidden_abort where
    ensures result == value + 1
    aborts_if value == 255

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let baseline := LeanerLang.Perf.parseBaseline
    (← IO.FS.readFile "LeanerLang/Tests/Performance.exp")
  let original := baseline.filter fun (target, _, _) =>
    target.startsWith "«0x42».perf_generics::carry_u64 "
  unless original.size == 2 do throwError "missing original generic-call cost ceiling"
  let heartbeats := original.foldl (fun total entry => total + entry.2.1) 0
  let objects := original.foldl (fun total entry => total + entry.2.2) 0
  for function in ["call_increment", "call_again", "bounded", "permitted", "reversed", "weak_caller"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_calls::{function} ")
    unless measured.size == 2 do throwError "missing native call measurements for {function}"
    let actualHeartbeats := measured.foldl (fun total entry => total + entry.heartbeats) 0
    let actualObjects := measured.foldl (fun total entry => total + entry.objects) 0
    logInfo m!"{function}: {actualHeartbeats} heartbeats / {actualObjects} objects"
    unless actualHeartbeats ≤ heartbeats && actualObjects ≤ objects do
      throwError "native call {function} exceeds original call ceiling {heartbeats} / {objects}"

#guard_msgs (drop error) in
#leaner_verify 0x42::native_calls::omitted_abort
#guard_msgs (drop error) in
#leaner_verify 0x42::native_calls::spurious_abort
#guard_msgs (drop error) in
#leaner_verify 0x42::native_calls::missing_precondition
#guard_msgs (drop error) in
#leaner_verify 0x42::native_calls::wrong_result
#guard_msgs (drop error) in
#leaner_verify 0x42::native_calls::hidden_abort

open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_calls
  for function in ["omitted_abort", "spurious_abort", "missing_precondition", "wrong_result", "hidden_abort"] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified] do
      let name := Name.str artifactRoot function ++ suffix
      if (← getEnv).contains name then throwError "rejected call leaked {name}"
    let input ← LeanerLang.Contract.prepareVerification Syntax.missing #["0x42", "native_calls"] function
    if (LeanerLang.NativeRegistry.entries.getState (← getEnv)).contains input.generated.relation then
      throwError "rejected call leaked its reusable registry entry"

-- Caller proofs must use the callee's published contract, never execution
-- agreement or unconditional semantic effect certificates. Audit their local
-- dependency closure, including callees and argument/result definitions.
open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_calls
  for (function, callee) in [("call_increment", "increment"), ("call_again", "call_increment"),
      ("bounded", "increment"), ("permitted", "bounded"), ("reversed", "add"),
      ("weak_caller", "unreported")] do
    let summary := Name.str artifactRoot function ++ `nativeSummary
    let some proof := (← getEnv).find? summary |>.bind (·.value? (allowOpaque := true))
      | throwError "missing call summary {summary}"
    unless proof.getUsedConstants.contains (Name.str artifactRoot callee ++ `nativeSummary) do
      throwError "{summary} does not reuse the callee contract"
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
          | throwError "missing native call artifact {name}"
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
            throwError "native call {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency
