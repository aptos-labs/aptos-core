-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Source verification defaults to native, rejects every retired route,
and cannot publish artifacts after a failed contract or generation gap. -/

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

leaner module 0x42::native_routes where
  fun missing(value : u64) -> u64 := value / 2
  spec missing where
    ensures result == value
    aborts_if false

  fun previous(value : u64) -> u64 := value
  spec previous where
    ensures result == value
    aborts_if false
  verify previous

  fun disabled(value : u64) -> u64 := value
  spec disabled where
    ensures result == value
    aborts_if false

  fun writes_parameter(value : &mut u64) -> Unit := *value := 1
  spec writes_parameter where
    ensures *value == 1
    aborts_if false
  verify writes_parameter

  fun requires_reborrow(value : &mut u64) -> Unit := do
    let view := &mut *value
    *view := 1
  spec requires_reborrow where
    ensures *value == 1
    aborts_if false

  fun cached(value : u64) -> u64 := value
  spec cached where
    ensures result == value
    aborts_if false

-- No explicit native option: both entry points use the native-only default.
#leaner_verify 0x42::native_routes::previous
#leaner_require_native 0x42::native_routes::previous
#leaner_require_native_all

/-- error: legacy verification route `script` is disabled; use `native` and migrate unsupported computations -/
#guard_msgs in
set_option leaner.route "script" in
#leaner_verify 0x42::native_routes::disabled

/-- error: legacy verification route `normalize` is disabled; use `native` and migrate unsupported computations -/
#guard_msgs in
set_option leaner.route "normalize" in
#leaner_verify 0x42::native_routes::disabled

/-- error: legacy verification route `compose` is disabled; use `native` and migrate unsupported computations -/
#guard_msgs in
set_option leaner.route "compose" in
#leaner_verify 0x42::native_routes::disabled

-- A cached native theorem cannot bypass the route check either.
/-- error: legacy verification route `compose` is disabled; use `native` and migrate unsupported computations -/
#guard_msgs in
set_option leaner.route "compose" in
#leaner_verify 0x42::native_routes::previous

-- Direct parameter mutation is native; source reborrowing is still a gap.
#leaner_require_native 0x42::native_routes::writes_parameter
#guard_msgs (drop error) in
#leaner_verify 0x42::native_routes::requires_reborrow

-- Division is supported, but this contract is false.
#guard_msgs (drop error) in
#leaner_verify 0x42::native_routes::missing

/-- error: native verification consumes computation certificates, not row scripts -/
#guard_msgs in
#leaner_verify 0x42::native_routes::disabled by trivial

open Lean Elab Command in
run_cmd do
  unless leaner.route.get ({} : Options) == "native" do
    throwError "source verification does not default to native"
  for function in [`missing, `disabled, `requires_reborrow] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_routes ++ function ++ suffix) then
        throwError "failed native verification leaked {function}.{suffix}"

-- An old cached name is not evidence of a native computation. This deliberate
-- malformed cache entry is admission-free and must never be reused.
theorem «0x42».native_routes.cached.typedVerified : True := True.intro

/-- error: `cached` was already verified by another route; the native route cannot reuse its frame/row proof -/
#guard_msgs in
#leaner_verify 0x42::native_routes::cached

/-- error: `cached` has no native computation transport; a frame/row typed theorem does not qualify -/
#guard_msgs in
#leaner_require_native 0x42::native_routes::cached

/-- error: `cached` has no native computation transport; a frame/row typed theorem does not qualify -/
#guard_msgs in
#leaner_require_native_all
