-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Negative/IntrinsicUnsupported. Intrinsic attributes are now
implemented: reject incomplete owners through the full Move schema and orphan
roles through source lowering, not the old blanket "unsupported" diagnostic.
A complete role graph is accepted and survives canonical source round-trip.
This checks schema support, not executable native-map semantics. -/

namespace LeanerE2ETests.Negative.IntrinsicUnsupported

open Lean Elab Command LeanerLang LeanerIR

private def sourceUnit (source : String) : CommandElabM CompilationUnit := do
  let (command, comments, namespaceDoc) ← match Print.Layout.namespaceStart source with
    | .ok parts => pure parts
    | .error reason => throwError "intrinsic source has no namespace: {reason}"
  let stx ← match Parser.runParserCategory (← getEnv) `command command with
    | .ok stx => pure stx
    | .error reason => throwError "intrinsic fixture did not parse: {reason}"
  match compilationUnitOfSyntax stx "<intrinsic diagnostic>" comments namespaceDoc with
  | .ok parsed => pure parsed
  | .error (_, reason) => throwError "intrinsic fixture did not elaborate: {reason}"

private def ownerSource : String :=
  "leaner module 0x42::intrinsic_diagnostics where\n" ++
  "  @[intrinsic_map]\n  struct Table {K} {V} where\n    marker : Bool\n"

private def roleSource : String :=
  "  @[map_spec_get (Table)]\n" ++
  "  opaque spec fun get {K} {V} (table : Table<K, V>, key : K) : V\n" ++
  "  @[map_spec_set (Table)]\n" ++
  "  opaque spec fun set {K} {V} (table : Table<K, V>, key : K, value : V) : Table<K, V>\n" ++
  "  @[map_spec_del (Table)]\n" ++
  "  opaque spec fun del {K} {V} (table : Table<K, V>, key : K) : Table<K, V>\n" ++
  "  @[map_spec_has_key (Table)]\n" ++
  "  opaque spec fun has_key {K} {V} (table : Table<K, V>, key : K) : Bool\n"

run_cmd do
  let owner ← sourceUnit ownerSource
  let raw ← match lower owner with
    | .ok raw => pure raw
    | Except.error diagnostics => throwError "intrinsic owner lowering failed: {repr diagnostics}"
  match Move.validate raw with
  | .ok _ => throwError "incomplete intrinsic owner was accepted by the Move schema"
  | Except.error diagnostics =>
    unless diagnostics.size == 4 && diagnostics.all (fun diagnostic =>
        diagnostic.code == "LIR-MOVE-INTRINSIC-ROLE-REQUIRED" &&
          diagnostic.primary.isSome) do
      throwError "wrong incomplete-owner diagnostic: {repr diagnostics}"

  let orphan ← sourceUnit <|
    "leaner module 0x42::orphan_intrinsic_role where\n" ++
    "  struct Table {K} {V} where\n    marker : Bool\n" ++
    "  @[map_spec_get (Table)]\n" ++
    "  opaque spec fun get {K} {V} (table : Table<K, V>, key : K) : V\n"
  match lower orphan with
  | .ok _ => throwError "orphan intrinsic role was accepted"
  | Except.error diagnostics =>
    unless diagnostics.size == 1 && diagnostics[0]!.code == "LEANER-ATTRIBUTE" &&
        diagnostics[0]!.message ==
          "attribute `map_spec_get` names `Table`, which has no intrinsic marker" &&
        diagnostics[0]!.span.isSome do
      throwError "wrong orphan-role diagnostic: {repr diagnostics}"

  let complete ← sourceUnit (ownerSource ++ roleSource)
  let raw ← match lower complete with
    | .ok raw => pure raw
    | Except.error diagnostics => throwError "complete intrinsic lowering failed: {repr diagnostics}"
  let unit ← match Move.validate raw with
    | .ok unit => pure unit
    | .error diagnostics => throwError "complete intrinsic graph rejected: {repr diagnostics}"
  let printed ← match Print.render (← getEnv) unit with
    | .ok value => pure value
    | .error reason => throwError "intrinsic printing failed: {repr reason}"
  let reparsed ← sourceUnit printed
  let raw ← match lower reparsed with
    | .ok raw => pure raw
    | Except.error diagnostics => throwError "intrinsic reimport failed: {repr diagnostics}"
  let reparsed ← match Move.validate raw with
    | .ok unit => pure unit
    | .error diagnostics => throwError "reimported graph rejected: {repr diagnostics}"
  let reprinted ← match Print.render (← getEnv) reparsed with
    | .ok value => pure value
    | .error reason => throwError "intrinsic reprinting failed: {repr reason}"
  unless printed == reprinted do throwError "intrinsic source is not a fixed point"

end LeanerE2ETests.Negative.IntrinsicUnsupported
