-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Cached verification must pass the same native-artifact audit as a fresh
denotation proof. A same-name declaration is not a verification certificate. -/

namespace LeanerLang.Tests.DenoteCache

set_option Elab.async false

leaner module 0x42::denote_cache where
  fun valid(value : u64) -> u64 := value
  spec valid where
    ensures result == value
    aborts_if false
  verify valid

  fun forged(value : u64) -> u64 := value
  spec forged where
    ensures result == value
    aborts_if false

-- Genuine certificates remain reusable, including inside a Lean namespace.
#leaner_verify 0x42::denote_cache::valid
#leaner_require_native 0x42::denote_cache::valid

theorem «0x42».denote_cache.forged.typedVerified : True := True.intro

/-- error: `forged` was verified by a retired route -/
#guard_msgs in
#leaner_verify 0x42::denote_cache::forged

open Lean Elab Command in
run_cmd do
  for suffix in [`compiled_eq, `verified] do
    if (← getEnv).contains
        (`LeanerLang.Tests.DenoteCache.«0x42».denote_cache.forged ++ suffix) then
      throwError "rejected cache entry leaked {suffix}"

end LeanerLang.Tests.DenoteCache

-- Deliberate untrusted test inputs, not admissions used to verify a program.
-- These exercise both direct axioms and dependencies outside artifact names.
set_option Elab.async false
open LeanerIR.Proofs.Denote

leaner module 0x42::forged_cache where
  fun direct() -> Unit := ()
  fun indirect() -> Unit := ()
  fun lookalike() -> Unit := ()

axiom «0x42».forged_cache.direct.typedVerified : @Term.denote = @Term.denote
theorem «0x42».forged_cache.direct.compiled : True := True.intro
theorem «0x42».forged_cache.direct.compiled_eq : True := True.intro

/-- error: artifact `«0x42».forged_cache.direct.typedVerified` depends on unapproved axiom `«0x42».forged_cache.direct.typedVerified` -/
#guard_msgs in
#leaner_verify 0x42::forged_cache::direct

axiom UntrustedCache.proof : @Term.denote = @Term.denote
theorem «0x42».forged_cache.indirect.typedVerified : @Term.denote = @Term.denote :=
  UntrustedCache.proof
theorem «0x42».forged_cache.indirect.compiled : True := True.intro
theorem «0x42».forged_cache.indirect.compiled_eq : True := True.intro

/-- error: artifact `«0x42».forged_cache.indirect.typedVerified` depends on unapproved axiom `UntrustedCache.proof` -/
#guard_msgs in
#leaner_verify 0x42::forged_cache::indirect

-- Even an axiom-free look-alike is not a generated contract proof.
theorem «0x42».forged_cache.lookalike.typedVerified : @Term.denote = @Term.denote := rfl
theorem «0x42».forged_cache.lookalike.compiled : True := True.intro
theorem «0x42».forged_cache.lookalike.compiled_eq : True := True.intro

/-- error: `lookalike` has no completed denotation verification -/
#guard_msgs in
#leaner_verify 0x42::forged_cache::lookalike

-- A private extension is not a security boundary for source metaprograms.
theorem «0x42».forged_cache.lookalike.verified : True := True.intro

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  let some name := env.constants.fold (init := none) fun found name _ =>
      if name.toString.endsWith "LeanerLang.Verify.completedDenotations" then some name else found
    | throwError "completion extension not found"
  let extension ← evalConst (checkMeta := false) TagDeclarationExtension name
  modifyEnv fun env => extension.tag env `«0x42».forged_cache.lookalike.typedVerified

/-- error: `lookalike` has an invalid public verification certificate -/
#guard_msgs in
#leaner_verify 0x42::forged_cache::lookalike

/-- error: `lookalike` has an invalid public verification certificate -/
#guard_msgs in
#leaner_require_native 0x42::forged_cache::lookalike
