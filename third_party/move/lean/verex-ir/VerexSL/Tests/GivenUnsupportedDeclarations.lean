-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! An otherwise supported generic identity uses the explicit concrete path
when another ADT in its unit needs a function-value invariant interpretation.
The unsupported declaration is checked before any Given proof starts. -/

verex module 0x42::given_unsupported_declarations where
  struct Operation has Copy, Drop where
    callback : Fn(u64) -> u64 has Copy, Drop

  fun identity {T}(value : T) -> T := value
  spec identity where
    aborts_if false
    ensures result == value

open Lean Elab Command VerexSL VerexSL.Contract VerexSL.Verify VerexIR.Proofs.Denote in
run_cmd do
  let env ← getEnv
  let base := `«0x42».given_unsupported_declarations.identity
  unless env.contains (base ++ `typedVerified) do
    throwError "unsupported Given declarations lost the existing concrete proof path"
  if env.contains (base ++ `givenTypedVerified) || env.contains (base ++ `givenTransfer) ||
      ((givenProofArtifacts.getState env).find? base).isSome then
    throwError "unsupported declarations were silently omitted from a Given proof"
  let some unit := registeredUnit? env base.getPrefix | throwError "missing source unit"
  let some (namespaceIndex, _, functionIndex, declaration) := findFunction? unit "identity"
    | throwError "missing source identity"
  let .ok compiled := compileFunction unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
    | throwError "the source identity did not compile"
  unless isGeneric declaration &&
      (givenUnsupported? unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ compiled).isNone do
    throwError "fixture must isolate unsupported unit declarations from supported body operations"

/-! A generic target that calls a callee without a declared contract takes the
concrete path as well: the Given interpretation can state an external callable
only as a `Satisfies` premise at its carrier family, and a callee with no
contract gives that premise nothing to say, so the caller could not establish
its own clauses from it. Concretely the callee is inlined instead. -/

verex module 0x42::given_contractless_callee where
  fun relay(value : u64) -> u64 := value

  fun relay_generic {T}(witness : T, value : u64) -> u64 := relay(value)
  spec relay_generic where
    aborts_if false
    ensures result == value

open Lean Elab Command VerexSL VerexSL.Contract VerexSL.Verify VerexIR.Proofs.Denote in
run_cmd do
  let env ← getEnv
  let base := `«0x42».given_contractless_callee.relay_generic
  unless env.contains (base ++ `typedVerified) do
    throwError "a contract-free callee lost the concrete proof path"
  if env.contains (base ++ `givenTypedVerified) || env.contains (base ++ `givenTransfer) ||
      ((givenProofArtifacts.getState env).find? base).isSome then
    throwError "a vacuous callee premise was stated in a Given proof"
  let some unit := registeredUnit? env base.getPrefix | throwError "missing source unit"
  let some (namespaceIndex, _, functionIndex, declaration) := findFunction? unit "relay_generic"
    | throwError "missing source relay_generic"
  let .ok compiled := compileFunction unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
    | throwError "the source relay_generic did not compile"
  unless isGeneric declaration do
    throwError "the fixture's target must be generic to reach the Given route"
  unless givenUnsupported? unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ compiled ==
      some "calls to callees without a contract" do
    throwError "a call of a contract-free callee is not reported as unsupported: \
      {repr (givenUnsupported? unit ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩ compiled)}"
