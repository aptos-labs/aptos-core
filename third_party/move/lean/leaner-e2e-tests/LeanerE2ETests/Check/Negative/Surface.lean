-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport
import LeanerRust.Profile

/-! Port of v0 Negative/Surface: invalid loop control and forged internal
markers fail at the source boundary. Move rejects duplicate active labels;
Rust may shadow them. Labels may be reused after their loop ends. -/

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for (label, body, code, message) in #[
      ("bare_break", "  fun test() -> Unit := do\n    break\n",
        "LEANER-BREAK-CONTEXT", "`break` appears outside a loop"),
      ("bare_continue", "  fun test() -> Unit := do\n    continue\n",
        "LEANER-CONTINUE-CONTEXT", "`continue` appears outside a loop"),
      ("unknown_label", "  fun test() -> Unit := do\n    loop@outer do\n      break@missing\n",
        "LEANER-LOOP-LABEL", "unknown loop label `missing`"),
      ("duplicate_label", "  fun test() -> Unit := do\n    loop@outer do\n      loop@outer do\n        break\n      break\n",
        "LEANER-LOOP-LABEL-DUPLICATE", "loop label `outer` is already used by an outer loop"),
      ("forged_marker", "  fun test() -> Unit := core.loopEnter(0, 0, 0, ())\n",
        "LEANER-LOCAL-NAME", "unknown local or constant `core`")] do
    let source := "leaner module 0x42::" ++ label ++ " where\n" ++ body
    let stx ← match Parser.runParserCategory env `command source with
      | .ok stx => pure stx
      | .error reason => throwError "surface fixture {label} did not parse: {reason}"
    let parsed ← match compilationUnitOfSyntax stx "<surface rejection>" with
      | .ok parsed => pure parsed
      | .error (_, reason) => throwError "surface fixture {label} did not elaborate: {reason}"
    match compile parsed with
    | .error (.frontend diagnostics) =>
      unless diagnostics.size == 1 && diagnostics[0]!.code == code &&
          diagnostics[0]!.message == message && diagnostics[0]!.span.isSome do
        throwError "wrong surface rejection for {label}: {repr diagnostics}"
    | .error reason => throwError "wrong rejection stage for {label}: {repr reason}"
    | .ok _ => throwError "invalid surface form {label} was accepted"

  -- A same-line operand must not be parsed as an unreachable second
  -- statement and then silently disappear during lowering.
  for suffix in [" test(n - 1)", " 1", " ()"] do
    let source := "leaner module 0x42::continue_call where\n" ++
      "  fun test(n : u64) -> u64 := do\n    while 0 < n do\n" ++
      "      continue" ++ suffix ++ "\n    return n\n"
    match Parser.runParserCategory env `command source with
    | .error reason =>
      unless reason.contains "`continue` takes no value or function call" do
        throwError "wrong continue-operand rejection: {reason}"
    | .ok _ => throwError "continue operand was accepted: {suffix}"

leaner module 0x42::surface_control where
  fun nested_exit() -> u64 := do
    let mut count : u64 := 0
    loop@outer do
      count := count + 1
      loop@inner do
        break@outer
    return count

  fun nested_continue() -> u64 := do
    let mut count : u64 := 0
    loop@outer do
      count := count + 1
      loop@inner do
        if count == 2 then break@outer
        continue@outer
    return count

  fun reused_label() -> u64 := do
    loop@again do
      break
    loop@again do
      break
    return 7

  fun branch_continue() -> u64 := do
    let mut count : u64 := 0
    loop do
      count := count + 1
      if count == 1 then continue else break
    return count

leaner namespace surface_control::rust using rust where
  fun shadowed_label() -> u64 := do
    let mut count : u64 := 0
    loop@same do
      loop@same do
        count := count + 1
        break@same
      count := count + 1
      break@same
    return count

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».surface_control #[
    ⟨"nested_exit", #[], .returned #[.integer 1], {}⟩,
    ⟨"nested_continue", #[], .returned #[.integer 2], {}⟩,
    ⟨"reused_label", #[], .returned #[.integer 7], {}⟩,
    ⟨"branch_continue", #[], .returned #[.integer 2], {}⟩]
  let some unit := LeanerLang.registeredUnit? (← Lean.getEnv) `surface_control.rust
    | throwError "missing Rust label-shadowing fixture"
  let executable ← match Validation.prepareExecution #[Rust.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "Rust label shadowing was rejected: {repr diagnostics}"
  match Interpreter.run executable 256 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] {} with
  | .ok (_, outcome) =>
    unless outcome.value == .returned #[.integer 2] do
      throwError "Rust label shadowing resolved to the wrong loop: {repr outcome.value}"
  | .error reason => throwError "Rust label shadowing failed: {repr reason}"

open Lean Elab Command LeanerLang in
run_cmd do
  for name in [`«0x42».surface_control, `surface_control.rust] do
    let env ← getEnv
    let some unit := registeredUnit? env name | throwError "missing control fixture {name}"
    let printed ← match Print.render env unit with
      | .ok value => pure value
      | .error reason => throwError "control printing failed: {repr reason}"
    let formatted ← match Print.formatSource env printed with
      | .ok value => pure value
      | .error reason => throwError "control reimport failed: {repr reason}"
    unless printed == formatted do throwError "control source is not a fixed point"
