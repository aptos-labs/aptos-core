-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"

#leaner_measure

leaner module 0x42::native_sequence where
  fun twice(value : u8) -> u8 := do
    let next := value + 1
    return next + 1
  spec twice where
    ensures result == value + 2
    aborts_if value >= 254
  verify twice

  fun four(value : u8) -> u8 := do
    let first := value + 1
    let second := first + 1
    let third := second + 1
    return third + 1
  spec four where
    ensures result == value + 4
    aborts_if value >= 252
  verify four

  fun eight(value : u8) -> u8 := do
    let a := value + 1
    let b := a + 1
    let c := b + 1
    let d := c + 1
    let e := d + 1
    let f := e + 1
    let g := f + 1
    return g + 1
  spec eight where
    ensures result == value + 8
    aborts_if value >= 248
  verify eight

  fun restore(value : u8) -> u8 := do
    let next := value - 1
    return next + 1
  spec restore where
    ensures result == value
    aborts_if value == 0
  verify restore

  fun bounded(value : u8) -> u8 := do
    let next := value + 1
    return next + 1
  spec bounded where
    requires value < 254
    ensures result == value + 2
    aborts_if false
  verify bounded

  fun mixed(left : u8, right : u8) -> u8 := do
    let difference := left - right
    return difference + right
  spec mixed where
    ensures result == left
    aborts_if left < right
  verify mixed

  fun down_up(value : u8) -> u8 := do
    let next := value - 1
    return next + 2
  spec down_up where
    ensures result == value + 1
    aborts_if value == 0 || value == 255
  verify down_up

  fun saved(value : u8) -> u8 := do
    let next := value + 1
    return next
  spec saved where
    ensures result == value + 1
    aborts_if value == 255
  verify saved

  fun discarded(value : u8) -> u8 := do
    let next := value + 1
    return value
  spec discarded where
    ensures result == value
    aborts_if value == 255
  verify discarded

  fun call_four(value : u8) -> u8 := core.call four::<>(value)
  spec call_four where
    ensures result == value + 4
    aborts_if value >= 252
  verify call_four

  -- Both initializer and continuation failures must be preserved.
  fun missing_second(value : u8) -> u8 := do
    let next := value + 1
    return next + 1
  spec missing_second where
    ensures result == value + 2
    aborts_if value == 255

  fun missing_first(value : u8) -> u8 := do
    let next := value + 1
    return next + 1
  spec missing_first where
    ensures result == value + 2
    aborts_if value == 254

  fun spurious(value : u8) -> u8 := do
    let next := value + 1
    return next + 1
  spec spurious where
    ensures result == value + 2
    aborts_if value >= 253

  fun wrong(value : u8) -> u8 := do
    let next := value + 1
    return next + 1
  spec wrong where
    ensures result == value + 1
    aborts_if value >= 254

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let baseline := LeanerLang.Perf.parseBaseline
    (← IO.FS.readFile "LeanerLang/Tests/Performance.exp")
  let total (function : String) : CommandElabM (Nat × Nat) := do
    let found := samples.filter (·.target.startsWith s!"«0x42».native_sequence::{function} ")
    unless found.size == 2 do throwError "missing native sequence measurements for {function}"
    return found.foldl (fun (heartbeats, objects) sample =>
      (heartbeats + sample.heartbeats, objects + sample.objects)) (0, 0)
  let ceiling (target : String) : CommandElabM (Nat × Nat) := do
    let found := baseline.filter (fun (name, _, _) => name.startsWith target)
    unless found.size == 2 do throwError "missing original ceiling {target}"
    return found.foldl (fun (heartbeats, objects) (_, h, o) => (heartbeats + h, objects + o)) (0, 0)
  let arithmetic ← ceiling "«0x42».perf_calls::guarded "
  let call ← ceiling "«0x42».perf_generics::carry_u64 "
  for function in ["twice", "four", "eight", "restore", "bounded", "mixed", "down_up",
      "saved", "discarded", "call_four"] do
    let actual ← total function
    let limit := if function == "call_four" then call else arithmetic
    let multiplier := if function == "eight" then 3 else if function == "four" then 2 else 1
    logInfo m!"{function}: {actual.1} heartbeats / {actual.2} objects (all generated stages)"
    unless actual.1 ≤ multiplier * limit.1 && actual.2 ≤ multiplier * limit.2 do
      throwError "native sequence {function} exceeds unchanged original arithmetic/call budgets"
  let two ← total "twice"
  let four ← total "four"
  let eight ← total "eight"
  -- Search must remain below quadratic growth; proof size must stay linear.
  unless four.1 ≤ 2 * two.1 && eight.1 ≤ 3 * four.1 &&
      four.2 ≤ 2 * two.2 && eight.2 ≤ 2 * four.2 do
    throwError "native sequence scaling regressed"

#guard_msgs (drop error) in
#leaner_verify 0x42::native_sequence::missing_second
#guard_msgs (drop error) in
#leaner_verify 0x42::native_sequence::missing_first
#guard_msgs (drop error) in
#leaner_verify 0x42::native_sequence::spurious
#guard_msgs (drop error) in
#leaner_verify 0x42::native_sequence::wrong

open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_sequence
  for function in ["missing_second", "missing_first", "spurious", "wrong"] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified] do
      let name := Name.str artifactRoot function ++ suffix
      if (← getEnv).contains name then throwError "rejected sequence leaked {name}"
    let input ← LeanerLang.Contract.prepareVerification Syntax.missing
      #["0x42", "native_sequence"] function
    if (LeanerLang.NativeRegistry.entries.getState (← getEnv)).contains input.generated.relation then
      throwError "rejected sequence leaked its reusable registry entry"
  for function in ["twice", "four", "eight", "restore", "bounded", "mixed", "down_up",
      "saved", "discarded", "call_four"] do
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
          | throwError "missing native sequence artifact {name}"
        let constants := declaration.type.getUsedConstants ++
          ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
        for dependency in constants do
          if dependency == ``LeanerIR.RuntimeFrame ||
              dependency == ``LeanerIR.Proofs.typedFunction ||
              dependency == ``LeanerIR.Proofs.decodeSpec ||
              (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
              (`LeanerIR.Proofs.ComputationAgreement).isPrefixOf dependency ||
              dependency.getString! == "computationRepresents" ||
              dependency.getString! == "computationState" || dependency == ``sorryAx ||
              (nativeValues && dependency == ``LeanerIR.RuntimeValue) then
            throwError "native sequence {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency
  let some caller := (← getEnv).find? (artifactRoot ++ `call_four.nativeSummary)
      |>.bind (·.value? (allowOpaque := true)) | throwError "missing modular sequence caller"
  unless caller.getUsedConstants.contains (artifactRoot ++ `four.nativeSummary) do
    throwError "sequence caller does not reuse the callee summary"

-- Exact payloads distinguish failure before the binding from failure in the
-- continuation. These check native execution, independently of a contract.
open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) :
    («0x42».native_sequence.down_up.computation ⟨⟨0, by decide⟩⟩).aborts
      state (.abort, #[.integer (-1)]) := by
  simp [«0x42».native_sequence.down_up.computation, Spec.bind, Spec.pure, Spec.abort,
    NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) :
    («0x42».native_sequence.down_up.computation ⟨⟨255, by decide⟩⟩).aborts
      state (.abort, #[.integer 256]) := by
  simp [«0x42».native_sequence.down_up.computation, Spec.bind, Spec.pure, Spec.abort,
    NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
