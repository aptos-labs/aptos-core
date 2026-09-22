-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Negative/Lowering at the current source/LIR boundary.
Host definitions cannot masquerade as source callees; metadata cannot replace
an authored body. Move recursive nominal types and continue operands reject.
Vector operations, specifications mentioning additional resource families,
and mutual calls retain positive coverage where v0's limitations are gone.
These are source semantics checks, not a compiler-correctness theorem. -/

set_option leaner.route "native"

def ordinaryHelper (value : Nat) : Nat := value + value

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for (name, body, stage, code) in #[
      ("recursive", "  struct RecursiveType where\n    next : RecursiveType\n",
        "lir", "LIR-MOVE-RECURSIVE-TYPE"),
      ("mutual_recursive", "  struct Left where\n    next : Vector<Right>\n" ++
        "  struct Right where\n    next : Left\n", "lir", "LIR-MOVE-RECURSIVE-TYPE"),
      ("omitted", "  fun calls(value : u64) -> u64 := omittedHelper(value)\n",
        "frontend", "LEANER-CALL-NAME"),
      ("host_helper", "  fun calls(value : u64) -> u64 := ordinaryHelper(value)\n",
        "frontend", "LEANER-CALL-NAME")] do
    let source := "leaner module 0x42::" ++ name ++ " where\n" ++ body
    let stx ← match Parser.runParserCategory env `command source with
      | .ok stx => pure stx
      | .error reason => throwError "lowering fixture did not parse: {reason}"
    let parsed ← match compilationUnitOfSyntax stx "<lowering diagnostic>" with
      | .ok parsed => pure parsed
      | .error (_, reason) => throwError "lowering fixture did not elaborate: {reason}"
    match compile parsed with
    | .ok _ => throwError "invalid lowering fixture {name} was accepted"
    | .error (.frontend diagnostics) =>
      unless stage == "frontend" && diagnostics.size == 1 &&
          diagnostics[0]!.code == code && diagnostics[0]!.span.isSome do
        throwError "wrong source rejection for {name}: {repr diagnostics}"
    | .error (.lir diagnostics) =>
      unless stage == "lir" && diagnostics.size == 1 &&
          diagnostics[0]!.code == code && diagnostics[0]!.primary.isSome do
        throwError "wrong LIR rejection for {name}: {repr diagnostics}"
  for body in ["value + continue test(value - 1)", "continue other(value)"] do
    let source := "leaner module 0x42::bad_continue where\n" ++
      "  fun test(value : u64) -> u64 := " ++ body ++ "\n"
    match Parser.runParserCategory env `command source with
    | .ok _ => throwError "legacy continue-call syntax was accepted"
    | .error reason =>
      unless reason.contains "`continue` takes no value or function call" do
        throwError "wrong continue-call rejection: {reason}"

leaner module 0x42::lowering_positive where
  @[move_source = "return 0"]
  fun authored_body() -> u64 := 9
  spec authored_body where
    ensures result == 9
    aborts_if false
  verify authored_body

  -- v0's Lean-host Vector.get helper is spelled as a checked indexed read
  -- in the current surface; `get` is not a Move standard-vector intrinsic.
  fun receiver_get(values : Vector<u64>, index : u64) -> u64 := do
    let element := &values[index]
    return *element
  spec receiver_get where
    ensures result == values[index]
    aborts_if !(index < values.length)
  verify receiver_get

  -- Each later bounds check needs the earlier guard in its context; a
  -- single retry of normalization is not enough for arbitrary sequencing.
  fun two_reads(left : Vector<u64>, right : Vector<u64>, index : u64) -> u64 := do
    let _first := &left[index]
    let second := &right[index]
    return *second
  spec two_reads where
    ensures result == right[index]
    aborts_if !(index < left.length) || !(index < right.length)
  verify two_reads

  fun receiver_insert() -> Unit := do
    let mut values := vector<u64>[1]
    let slot := &mut values
    -- As in the VectorOperations port, spell the builtin through shared LIR;
    -- an unrelated Lean-host Vector.insert definition cannot supply its body.
    *slot := core.prim.insertVector(*slot, 0, 7)
  spec receiver_insert where
    ensures true
    aborts_if false
  verify receiver_insert

  struct Vault has Key where
    value : u64
  struct Other has Key where
    value : u64
  fun touches_vault(address : Address) -> Bool := exists<Vault>(address)
  spec touches_vault where
    requires exists<Other>(address)
    ensures exists<Other>(address)
    aborts_if false
  verify touches_vault

  fun ping(value : u64) -> u64 := if value == 0 then 0 else pong(value - 1)
  fun pong(value : u64) -> u64 := if value == 0 then 1 else ping(value - 1)
  fun calls_mutual(value : u64) -> u64 := ping(value)
  spec calls_mutual where
    ensures true
    aborts_if false

leaner namespace lowering::rust using rust where
  struct RecursiveType where
    next : Vector<RecursiveType>

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let bounds : ThrowKind := .profile { profile := .move, tag := "runtime.vector_error" }
  assertRuns `«0x42».lowering_positive #[
    ⟨"authored_body", #[], .returned #[.integer 9], {}⟩,
    ⟨"receiver_get", #[.vector #[.integer 5, .integer 8], .integer 1], .returned #[.integer 8], {}⟩,
    ⟨"receiver_get", #[.vector #[], .integer 0], .threw bounds #[.integer 1], {}⟩,
    ⟨"receiver_get", #[.vector #[.integer 5], .integer 1], .threw bounds #[.integer 1], {}⟩,
    ⟨"two_reads", #[.vector #[.integer 5], .vector #[.integer 8], .integer 0],
      .returned #[.integer 8], {}⟩,
    ⟨"two_reads", #[.vector #[], .vector #[.integer 8], .integer 0],
      .threw bounds #[.integer 1], {}⟩,
    ⟨"two_reads", #[.vector #[.integer 5], .vector #[], .integer 0],
      .threw bounds #[.integer 1], {}⟩,
    ⟨"receiver_insert", #[], .returned #[], {}⟩,
    ⟨"touches_vault", #[.address "0x1"], .returned #[.bool false], {}⟩,
    ⟨"calls_mutual", #[.integer 0], .returned #[.integer 0], {}⟩,
    ⟨"calls_mutual", #[.integer 1], .returned #[.integer 1], {}⟩,
    ⟨"calls_mutual", #[.integer 2], .returned #[.integer 0], {}⟩,
    ⟨"calls_mutual", #[.integer 3], .returned #[.integer 1], {}⟩]

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for function in ["authored_body", "receiver_get", "two_reads", "receiver_insert", "touches_vault"] do
    let proof := ((`«0x42».lowering_positive).str function).str "verified"
    unless env.contains proof do throwError "missing lowering proof: {proof}"
    if (← collectAxioms proof).contains ``sorryAx then
      throwError "lowering proof contains an admission: {proof}"
  for name in [`«0x42».lowering_positive, `lowering.rust] do
    let some unit := registeredUnit? env name | throwError "missing lowering fixture"
    let printed ← match Print.render env unit with
      | .ok value => pure value
      | .error reason => throwError "lowering printing failed: {repr reason}"
    let formatted ← match Print.formatSource env printed with
      | .ok value => pure value
      | .error reason => throwError "lowering reimport failed: {repr reason}"
    unless printed == formatted do throwError "lowering source is not a fixed point"
